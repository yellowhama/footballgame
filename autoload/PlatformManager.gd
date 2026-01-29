extends Node
## Platform detection and management for cross-platform UI
## Detects mobile/tablet/desktop and portrait/landscape
## Phase 7A Foundation Component

enum Platform { MOBILE, TABLET, DESKTOP }
enum Orientation { PORTRAIT, LANDSCAPE }

var current_platform := Platform.MOBILE
var current_orientation := Orientation.PORTRAIT
var viewport_size: Vector2i = Vector2i(1080, 1920)
var aspect_ratio: float = 0.5625
var dpi: int = 96

signal platform_changed(new_platform: Platform)
signal orientation_changed(new_orientation: Orientation)
signal viewport_resized(new_size: Vector2i)


func _ready():
	print("[PlatformManager] Initializing...")
	get_tree().root.size_changed.connect(_on_viewport_changed)
	_detect_platform()
	print(
		(
			"[PlatformManager] Initialized - Platform: %s, Orientation: %s"
			% [Platform.keys()[current_platform], Orientation.keys()[current_orientation]]
		)
	)


func _detect_platform():
	# Get current viewport size
	viewport_size = get_viewport().size
	aspect_ratio = float(viewport_size.x) / float(viewport_size.y)

	# Get screen DPI (fallback to 96 if not available)
	dpi = DisplayServer.screen_get_dpi()
	if dpi <= 0:
		dpi = 96

	# Platform classification: Use viewport width first (more reliable for testing)
	# Then fall back to diagonal calculation for real devices
	var width = viewport_size.x

	# Viewport width-based classification (for testing and responsive design)
	if width < 768:
		current_platform = Platform.MOBILE
	elif width < 1280:
		current_platform = Platform.TABLET
	else:
		current_platform = Platform.DESKTOP

	# Orientation detection
	if aspect_ratio < 1.0:
		current_orientation = Orientation.PORTRAIT
	else:
		current_orientation = Orientation.LANDSCAPE

	print(
		(
			"[PlatformManager] Detected - Size: %s, Aspect: %.2f, Width: %dpx → %s %s"
			% [
				viewport_size,
				aspect_ratio,
				width,
				Platform.keys()[current_platform],
				Orientation.keys()[current_orientation]
			]
		)
	)


func _calculate_diagonal_inches(width: int, height: int, screen_dpi: int) -> float:
	if screen_dpi <= 0:
		screen_dpi = 96  # Fallback

	var width_inches = float(width) / float(screen_dpi)
	var height_inches = float(height) / float(screen_dpi)
	return sqrt(width_inches * width_inches + height_inches * height_inches)


func _on_viewport_changed():
	var old_platform = current_platform
	var old_orientation = current_orientation
	var old_size = viewport_size

	_detect_platform()

	# Emit signals for changes
	if old_size != viewport_size:
		viewport_resized.emit(viewport_size)

	if old_platform != current_platform:
		print(
			(
				"[PlatformManager] Platform changed: %s → %s"
				% [Platform.keys()[old_platform], Platform.keys()[current_platform]]
			)
		)
		platform_changed.emit(current_platform)

	if old_orientation != current_orientation:
		print(
			(
				"[PlatformManager] Orientation changed: %s → %s"
				% [Orientation.keys()[old_orientation], Orientation.keys()[current_orientation]]
			)
		)
		orientation_changed.emit(current_orientation)


## Helper functions for platform checks
func is_mobile() -> bool:
	return current_platform == Platform.MOBILE


func is_tablet() -> bool:
	return current_platform == Platform.TABLET


func is_desktop() -> bool:
	return current_platform == Platform.DESKTOP


func is_portrait() -> bool:
	return current_orientation == Orientation.PORTRAIT


func is_landscape() -> bool:
	return current_orientation == Orientation.LANDSCAPE


func get_platform_name() -> String:
	return Platform.keys()[current_platform]


func get_orientation_name() -> String:
	return Orientation.keys()[current_orientation]


## Breakpoint helpers (Phase 7 Spec Section 2.3)
func get_layout_variant() -> String:
	var width = viewport_size.x

	if width < 480:
		return "mobile_small"  # < 480px
	elif width < 768:
		return "mobile_large"  # 480-768px
	elif width < 1024:
		return "tablet_portrait"  # 768-1024px
	elif width < 1280:
		return "tablet_landscape"  # 1024-1280px
	elif width < 1920:
		return "desktop_hd"  # 1280-1920px
	elif width < 2560:
		return "desktop_fhd"  # 1920-2560px
	else:
		return "desktop_qhd"  # 2560px+


func should_use_mobile_layout() -> bool:
	return is_portrait() and (is_mobile() or is_tablet())


func should_use_desktop_layout() -> bool:
	return is_desktop() or (is_tablet() and is_landscape())
