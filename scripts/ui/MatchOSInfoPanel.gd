extends PanelContainer
class_name MatchOSInfoPanel

signal panel_closed

var _data: Dictionary = {}


func _ready() -> void:
	# Center on screen
	set_anchors_preset(Control.PRESET_CENTER)
	custom_minimum_size = Vector2(500, 400)

	# Panel style
	var style = StyleBoxFlat.new()
	style.bg_color = Color(0.15, 0.15, 0.15, 0.95)
	style.corner_radius_top_left = 12
	style.corner_radius_top_right = 12
	style.corner_radius_bottom_left = 12
	style.corner_radius_bottom_right = 12
	style.border_width_all = 2
	style.border_color = Color(0.3, 0.3, 0.3)
	add_theme_stylebox_override("panel", style)

	_build_ui()

	# Fade in animation
	modulate.a = 0.0
	scale = Vector2(0.95, 0.95)
	var tween = create_tween()
	tween.set_parallel(true)
	tween.tween_property(self, "modulate:a", 1.0, 0.25).set_ease(Tween.EASE_OUT)
	tween.tween_property(self, "scale", Vector2(1.0, 1.0), 0.25).set_ease(Tween.EASE_OUT).set_trans(Tween.TRANS_BACK)


func _build_ui() -> void:
	var margin = MarginContainer.new()
	margin.add_theme_constant_override("margin_left", 20)
	margin.add_theme_constant_override("margin_top", 20)
	margin.add_theme_constant_override("margin_right", 20)
	margin.add_theme_constant_override("margin_bottom", 20)
	add_child(margin)

	var vbox = VBoxContainer.new()
	vbox.add_theme_constant_override("separation", 12)
	margin.add_child(vbox)

	# Title bar
	var title_bar = HBoxContainer.new()
	vbox.add_child(title_bar)

	var title = Label.new()
	title.text = "Match OS Debug Panel"
	title.add_theme_font_size_override("font_size", 24)
	title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	title_bar.add_child(title)

	var close_btn = Button.new()
	close_btn.text = "X"
	close_btn.custom_minimum_size = Vector2(40, 40)
	close_btn.pressed.connect(_on_close_pressed)
	title_bar.add_child(close_btn)

	# Separator
	var sep1 = HSeparator.new()
	vbox.add_child(sep1)

	# Content container
	var content_box = VBoxContainer.new()
	content_box.name = "ContentBox"
	content_box.add_theme_constant_override("separation", 8)
	vbox.add_child(content_box)

	# Will be populated by update_data()


func update_data(data: Dictionary) -> void:
	_data = data
	_update_display()


func _update_display() -> void:
	var content_box = get_node_or_null("MarginContainer/VBoxContainer/ContentBox")
	if not content_box:
		return

	# Clear old content
	for child in content_box.get_children():
		child.queue_free()

	# Add pressure row
	if _data.has("local_pressure"):
		_add_data_row(content_box, "Local Pressure", "%.2f" % _data["local_pressure"])

	# Add cell row
	if _data.has("cell"):
		var cell = _data["cell"]
		_add_data_row(content_box, "Cell", "(%d, %d)" % [cell.x, cell.y])

	# Add track_id row
	if _data.has("track_id"):
		_add_data_row(content_box, "Track ID", str(_data["track_id"]))


func _add_data_row(parent: VBoxContainer, label_text: String, value_text: String) -> void:
	var row = HBoxContainer.new()

	var label = Label.new()
	label.text = label_text + ":"
	label.custom_minimum_size.x = 150
	label.add_theme_font_size_override("font_size", 16)
	row.add_child(label)

	var value = Label.new()
	value.text = value_text
	value.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	value.add_theme_font_size_override("font_size", 16)
	value.add_theme_color_override("font_color", Color(0.3, 0.8, 1.0))
	row.add_child(value)

	parent.add_child(row)


func _on_close_pressed() -> void:
	var tween = create_tween()
	tween.set_parallel(true)
	tween.tween_property(self, "modulate:a", 0.0, 0.15).set_ease(Tween.EASE_IN)
	tween.tween_property(self, "scale", Vector2(0.95, 0.95), 0.15).set_ease(Tween.EASE_IN)
	await tween.finished

	panel_closed.emit()
	queue_free()


func _input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed and event.keycode == KEY_ESCAPE:
		_on_close_pressed()
