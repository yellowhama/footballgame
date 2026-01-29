# In-game performance HUD overlay
extends Control
class_name ProfilerOverlay

@onready var _perf := PerfCounters.new()
var _enabled := true
var _font: Font


func _ready():
	add_child(_perf)

	# Position in top-left corner
	position = Vector2(8, 8)
	size = Vector2(200, 100)
	z_index = 1000  # High z-index to appear on top

	# Make sure it's not affected by scene scaling
	top_level = true

	# Try to get default font
	_font = get_theme_default_font()

	print("[ProfilerOverlay] Performance HUD enabled")


func _process(_delta):
	if _enabled:
		queue_redraw()


func _draw():
	if not _enabled:
		return

	var s := _perf.stats()

	# Background
	draw_rect(Rect2(Vector2.ZERO, Vector2(190, 70)), Color(0, 0, 0, 0.7), true)

	# Text color
	var text_color = Color.WHITE
	var font_size = 12

	# FPS
	var fps_text = "FPS: %0.1f" % s.fps
	var fps_color = Color.GREEN if s.fps >= 30 else (Color.YELLOW if s.fps >= 15 else Color.RED)
	draw_string(_font, Vector2(8, 18), fps_text, HORIZONTAL_ALIGNMENT_LEFT, -1, font_size, fps_color)

	# Frame time
	var frametime_text = "Frame: %0.2fms" % (s.avg_dt * 1000)
	draw_string(_font, Vector2(8, 34), frametime_text, HORIZONTAL_ALIGNMENT_LEFT, -1, font_size, text_color)

	# Memory (if available)
	if s.has("memory_mb"):
		var memory_text = "Mem: %0.1fMB" % s.memory_mb
		draw_string(_font, Vector2(8, 50), memory_text, HORIZONTAL_ALIGNMENT_LEFT, -1, font_size, text_color)

	# Frame count
	var frame_text = "Frames: %d" % s.frame_count
	draw_string(_font, Vector2(8, 66), frame_text, HORIZONTAL_ALIGNMENT_LEFT, -1, font_size, Color.GRAY)


func toggle() -> void:
	_enabled = not _enabled
	visible = _enabled


func set_enabled(enabled: bool) -> void:
	_enabled = enabled
	visible = _enabled


func is_enabled() -> bool:
	return _enabled


func get_stats() -> Dictionary:
	return _perf.stats()


# Handle input to toggle overlay
func _input(event):
	# Toggle with F3 key
	if event is InputEventKey and event.pressed:
		if event.keycode == KEY_F3:
			toggle()
