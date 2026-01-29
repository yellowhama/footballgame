class_name AdaptiveLayoutContainer
extends Control
## Adaptive layout container for cross-platform responsive UI
## Phase 7A Foundation Component - Base class for responsive scenes
##
## Usage:
## 1. Create scene with AdaptiveLayoutContainer as root
## 2. Add 3 children: MobilePortraitLayout, TabletHybridLayout, DesktopLandscapeLayout
## 3. Design each layout variant independently
## 4. Container automatically shows/hides appropriate layout based on platform
##
## Example structure:
## AdaptiveLayoutContainer (this script)
## ├─ MobilePortraitLayout (Node, visible on mobile)
## │  └─ VBoxContainer
## │     ├─ Header
## │     ├─ Content
## │     └─ Footer
## ├─ TabletHybridLayout (Node, visible on tablet)
## │  └─ GridContainer
## │     └─ Flexible grid content
## └─ DesktopLandscapeLayout (Node, visible on desktop)
##    └─ HBoxContainer
##       ├─ Sidebar
##       └─ MainContent

## Layout node references (set via node names)
@onready var mobile_layout: CanvasItem = get_node_or_null("MobilePortraitLayout")
@onready var tablet_layout: CanvasItem = get_node_or_null("TabletHybridLayout")
@onready var desktop_layout: CanvasItem = get_node_or_null("DesktopLandscapeLayout")

## Current active layout reference
var active_layout: CanvasItem = null

## Signals for layout activation events
signal layout_activated(layout_name: String)
signal mobile_layout_activated
signal tablet_layout_activated
signal desktop_layout_activated


func _ready():
	# Ensure we fill the viewport
	set_anchors_and_offsets_preset(PRESET_FULL_RECT)

	# Connect to PlatformManager signals
	if PlatformManager:
		PlatformManager.platform_changed.connect(_on_platform_changed)
		PlatformManager.orientation_changed.connect(_on_orientation_changed)
		PlatformManager.viewport_resized.connect(_on_viewport_resized)

		# Wait one frame for children to be ready
		await get_tree().process_frame

		# Initial layout update
		_update_layout()

		print("[AdaptiveLayoutContainer] Initialized - Active layout: %s" % get_active_layout_name())
	else:
		push_error("[AdaptiveLayoutContainer] PlatformManager not found! Cannot detect platform.")


func _on_platform_changed(_new_platform: PlatformManager.Platform):
	_update_layout()


func _on_orientation_changed(_new_orientation: PlatformManager.Orientation):
	_update_layout()


func _on_viewport_resized(_new_size: Vector2i):
	# Viewport resize may not always trigger platform change, but we recheck
	_update_layout()


func _update_layout():
	"""Switch visible layout based on current platform"""
	if not PlatformManager:
		return

	var platform = PlatformManager.current_platform
	var new_active: Node = null
	var layout_name: String = ""

	# Determine which layout should be active
	match platform:
		PlatformManager.Platform.MOBILE:
			new_active = mobile_layout
			layout_name = "mobile"
		PlatformManager.Platform.TABLET:
			new_active = tablet_layout
			layout_name = "tablet"
		PlatformManager.Platform.DESKTOP:
			new_active = desktop_layout
			layout_name = "desktop"

	# Validate layout node exists
	if not new_active:
		push_warning(
			(
				"[AdaptiveLayoutContainer] Layout node '%s' not found! Add a child node with exact name."
				% _get_expected_node_name(platform)
			)
		)
		return

	# If layout hasn't changed, skip update
	if new_active == active_layout:
		return

	# Hide all layouts
	if mobile_layout:
		mobile_layout.visible = false
	if tablet_layout:
		tablet_layout.visible = false
	if desktop_layout:
		desktop_layout.visible = false

	# Show and activate new layout
	new_active.visible = true
	active_layout = new_active

	# Emit signals
	layout_activated.emit(layout_name)
	match platform:
		PlatformManager.Platform.MOBILE:
			mobile_layout_activated.emit()
		PlatformManager.Platform.TABLET:
			tablet_layout_activated.emit()
		PlatformManager.Platform.DESKTOP:
			desktop_layout_activated.emit()

	print(
		(
			"[AdaptiveLayoutContainer] Layout switched to: %s (Platform: %s, Orientation: %s)"
			% [layout_name, PlatformManager.get_platform_name(), PlatformManager.get_orientation_name()]
		)
	)

	# Call layout-specific activation callback if it exists
	if new_active.has_method("_on_layout_activated"):
		new_active._on_layout_activated()


## Public API


func get_active_layout() -> Node:
	"""Get the currently active layout node"""
	return active_layout


func get_active_layout_name() -> String:
	"""Get the name of currently active layout"""
	if active_layout == mobile_layout:
		return "mobile"
	elif active_layout == tablet_layout:
		return "tablet"
	elif active_layout == desktop_layout:
		return "desktop"
	else:
		return "none"


func is_mobile_active() -> bool:
	return active_layout == mobile_layout


func is_tablet_active() -> bool:
	return active_layout == tablet_layout


func is_desktop_active() -> bool:
	return active_layout == desktop_layout


func force_layout_update():
	"""Manually trigger layout update (useful after dynamic layout changes)"""
	_update_layout()


## Helper functions


func _get_expected_node_name(platform: PlatformManager.Platform) -> String:
	"""Get expected child node name for platform"""
	match platform:
		PlatformManager.Platform.MOBILE:
			return "MobilePortraitLayout"
		PlatformManager.Platform.TABLET:
			return "TabletHybridLayout"
		PlatformManager.Platform.DESKTOP:
			return "DesktopLandscapeLayout"
		_:
			return "UnknownLayout"


## Validation helpers


func validate_layout_structure() -> Dictionary:
	"""
	Validate that all required layout nodes exist
	Returns: { "valid": bool, "missing": Array[String], "warnings": Array[String] }
	"""
	var result = {"valid": true, "missing": [], "warnings": []}

	if not mobile_layout:
		result.missing.append("MobilePortraitLayout")
		result.valid = false

	if not tablet_layout:
		result.missing.append("TabletHybridLayout")
		result.valid = false

	if not desktop_layout:
		result.missing.append("DesktopLandscapeLayout")
		result.valid = false

	# Check if layouts have content
	if mobile_layout and mobile_layout.get_child_count() == 0:
		result.warnings.append("MobilePortraitLayout has no children")

	if tablet_layout and tablet_layout.get_child_count() == 0:
		result.warnings.append("TabletHybridLayout has no children")

	if desktop_layout and desktop_layout.get_child_count() == 0:
		result.warnings.append("DesktopLandscapeLayout has no children")

	return result


func validate_ui_standards_base():
	"""
	Base implementation of UIStandards validation
	Checks touch target sizes and font sizes, auto-fixes violations if enabled
	Child screens should call this from their _validate_ui_standards() method
	"""
	if not UIStandards or not UIStandards.validation_enabled:
		return

	var screen_name = get_script().get_path().get_file().get_basename() if get_script() else "AdaptiveLayoutContainer"
	print("[%s] Running UIStandards validation..." % screen_name)

	var violations = UIStandards.scan_scene_for_violations(self)

	if violations.touch_targets.size() > 0:
		print("[%s] Touch target violations found: %d" % [screen_name, violations.touch_targets.size()])
		for node in violations.touch_targets:
			print("  - %s (size: %v)" % [node.name, node.size])
			# Auto-fix if enabled
			if UIStandards.auto_fix_touch_target(node):
				print("    ✓ Auto-fixed to: %v" % node.custom_minimum_size)

	if violations.font_sizes.size() > 0:
		print("[%s] Font size violations found: %d" % [screen_name, violations.font_sizes.size()])
		for node in violations.font_sizes:
			print("  - %s" % node.name)

	if violations.touch_targets.size() == 0 and violations.font_sizes.size() == 0:
		print("[%s] ✅ UIStandards validation passed" % screen_name)


func print_layout_info():
	"""Debug helper: print current layout state"""
	print("[AdaptiveLayoutContainer] Layout Info:")
	print("  Active: %s" % get_active_layout_name())
	print("  Mobile: %s (visible: %s)" % [mobile_layout != null, mobile_layout.visible if mobile_layout else "N/A"])
	print("  Tablet: %s (visible: %s)" % [tablet_layout != null, tablet_layout.visible if tablet_layout else "N/A"])
	print("  Desktop: %s (visible: %s)" % [desktop_layout != null, desktop_layout.visible if desktop_layout else "N/A"])
	print("  Platform: %s" % PlatformManager.get_platform_name() if PlatformManager else "N/A")
	print("  Viewport: %s" % PlatformManager.viewport_size if PlatformManager else "N/A")
