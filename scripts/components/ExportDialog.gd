extends AcceptDialog
class_name ExportDialog
## Export dialog for history data
## Phase 13: Extended Features - Data Export UI

signal export_requested(export_type: String, format: String, file_path: String)

## Export type options
enum ExportType { MATCH_RECORDS, TRAINING_RECORDS, ALL_RECORDS }

## Format options
enum ExportFormat { CSV, JSON }

## UI References
var export_type_option: OptionButton
var format_option: OptionButton
var file_path_input: LineEdit
var browse_button: Button
var file_dialog: FileDialog
var status_label: Label
var cancel_button: Button = null

## Current selections
var current_export_type: ExportType = ExportType.ALL_RECORDS
var current_format: ExportFormat = ExportFormat.JSON


func _init():
	# Dialog setup
	title = "데이터 내보내기"
	dialog_autowrap = true
	size = Vector2i(600, 400)
	ok_button_text = "내보내기"

	_build_ui()
	_connect_signals()


func _ready() -> void:
	if cancel_button == null:
		cancel_button = add_cancel_button("취소")
	else:
		cancel_button.text = "취소"


func _build_ui():
	# Main container
	var vbox = VBoxContainer.new()
	vbox.add_theme_constant_override("separation", 16)
	add_child(vbox)

	# Export type section
	var type_label = Label.new()
	type_label.text = "내보내기 유형:"
	type_label.add_theme_font_size_override("font_size", 16)
	vbox.add_child(type_label)

	export_type_option = OptionButton.new()
	export_type_option.add_item("경기 기록만", ExportType.MATCH_RECORDS)
	export_type_option.add_item("훈련 기록만", ExportType.TRAINING_RECORDS)
	export_type_option.add_item("모든 기록 (경기 + 훈련)", ExportType.ALL_RECORDS)
	export_type_option.selected = ExportType.ALL_RECORDS
	export_type_option.custom_minimum_size = Vector2(0, 48)
	vbox.add_child(export_type_option)

	# Format section
	var format_label = Label.new()
	format_label.text = "파일 형식:"
	format_label.add_theme_font_size_override("font_size", 16)
	vbox.add_child(format_label)

	format_option = OptionButton.new()
	format_option.add_item("JSON (구조화된 데이터)", ExportFormat.JSON)
	format_option.add_item("CSV (스프레드시트용)", ExportFormat.CSV)
	format_option.selected = ExportFormat.JSON
	format_option.custom_minimum_size = Vector2(0, 48)
	vbox.add_child(format_option)

	# File path section
	var path_label = Label.new()
	path_label.text = "저장 위치:"
	path_label.add_theme_font_size_override("font_size", 16)
	vbox.add_child(path_label)

	var path_hbox = HBoxContainer.new()
	path_hbox.add_theme_constant_override("separation", 8)
	vbox.add_child(path_hbox)

	file_path_input = LineEdit.new()
	file_path_input.placeholder_text = "파일 경로를 선택하세요..."
	file_path_input.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	file_path_input.custom_minimum_size = Vector2(0, 48)
	file_path_input.editable = false  # Read-only, use browse button
	path_hbox.add_child(file_path_input)

	browse_button = Button.new()
	browse_button.text = "찾아보기"
	browse_button.custom_minimum_size = Vector2(120, 48)
	path_hbox.add_child(browse_button)

	# Status label
	status_label = Label.new()
	status_label.text = ""
	status_label.add_theme_color_override("font_color", Color(0.7, 0.7, 0.7))
	status_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	vbox.add_child(status_label)

	# File dialog (hidden)
	file_dialog = FileDialog.new()
	file_dialog.file_mode = FileDialog.FILE_MODE_SAVE_FILE
	file_dialog.access = FileDialog.ACCESS_FILESYSTEM
	file_dialog.use_native_dialog = true
	file_dialog.add_filter("*.json", "JSON Files")
	file_dialog.add_filter("*.csv", "CSV Files")
	add_child(file_dialog)

	# Set initial suggested filename
	_update_suggested_filename()


func _connect_signals():
	if export_type_option:
		export_type_option.item_selected.connect(_on_export_type_changed)

	if format_option:
		format_option.item_selected.connect(_on_format_changed)

	if browse_button:
		browse_button.pressed.connect(_on_browse_pressed)

	if file_dialog:
		file_dialog.file_selected.connect(_on_file_selected)

	# OK button (from AcceptDialog)
	confirmed.connect(_on_export_confirmed)


func _on_export_type_changed(index: int):
	"""Export type selection changed"""
	current_export_type = export_type_option.get_item_id(index)
	_update_suggested_filename()


func _on_format_changed(index: int):
	"""Format selection changed"""
	current_format = format_option.get_item_id(index)
	_update_suggested_filename()
	_update_file_dialog_filters()


func _on_browse_pressed():
	"""Browse button clicked"""
	if file_dialog:
		file_dialog.popup_centered(Vector2i(800, 600))


func _on_file_selected(path: String):
	"""File path selected from dialog"""
	file_path_input.text = path
	_validate_file_path()


func _on_export_confirmed():
	"""Export button (OK) clicked"""
	var file_path = file_path_input.text

	# Validate path
	if file_path == "":
		status_label.text = "⚠️ 파일 경로를 선택하세요"
		status_label.add_theme_color_override("font_color", Color(1.0, 0.5, 0.5))
		return

	var validation = ExportService.validate_export_path(file_path)
	if not validation.valid:
		status_label.text = "⚠️ %s" % validation.error
		status_label.add_theme_color_override("font_color", Color(1.0, 0.5, 0.5))
		return

	# Get export type string
	var export_type_str = ""
	match current_export_type:
		ExportType.MATCH_RECORDS:
			export_type_str = "match"
		ExportType.TRAINING_RECORDS:
			export_type_str = "training"
		ExportType.ALL_RECORDS:
			export_type_str = "all"

	# Get format string
	var format_str = "json" if current_format == ExportFormat.JSON else "csv"

	# Emit export request
	export_requested.emit(export_type_str, format_str, file_path)

	# Close dialog
	hide()


func _update_suggested_filename():
	"""Update suggested filename based on current selections"""
	if not ExportService:
		return

	var export_type_str = ""
	match current_export_type:
		ExportType.MATCH_RECORDS:
			export_type_str = "match_records"
		ExportType.TRAINING_RECORDS:
			export_type_str = "training_records"
		ExportType.ALL_RECORDS:
			export_type_str = "all_records"

	var format_str = "json" if current_format == ExportFormat.JSON else "csv"

	var suggested_name = ExportService.get_suggested_filename(export_type_str, format_str)

	# Set as default filename in file dialog
	if file_dialog:
		file_dialog.current_file = suggested_name

	# Update status
	status_label.text = "제안된 파일명: %s" % suggested_name
	status_label.add_theme_color_override("font_color", Color(0.5, 0.8, 1.0))


func _update_file_dialog_filters():
	"""Update file dialog filters based on format"""
	if not file_dialog:
		return

	file_dialog.clear_filters()

	if current_format == ExportFormat.JSON:
		file_dialog.add_filter("*.json", "JSON Files")
	else:
		file_dialog.add_filter("*.csv", "CSV Files")


func _validate_file_path():
	"""Validate selected file path"""
	var file_path = file_path_input.text

	if file_path == "":
		return

	var validation = ExportService.validate_export_path(file_path)

	if validation.valid:
		if validation.error != "":
			status_label.text = "⚠️ %s" % validation.error
			status_label.add_theme_color_override("font_color", Color(1.0, 0.8, 0.3))
		else:
			status_label.text = "✅ 경로가 유효합니다"
			status_label.add_theme_color_override("font_color", Color(0.5, 1.0, 0.5))
	else:
		status_label.text = "❌ %s" % validation.error
		status_label.add_theme_color_override("font_color", Color(1.0, 0.5, 0.5))


## Public API


func show_dialog():
	"""Show the export dialog"""
	_update_suggested_filename()
	popup_centered()


func set_default_export_type(export_type: ExportType):
	"""Set default export type"""
	current_export_type = export_type
	if export_type_option:
		export_type_option.selected = export_type
	_update_suggested_filename()


func set_default_format(format: ExportFormat):
	"""Set default format"""
	current_format = format
	if format_option:
		format_option.selected = format
	_update_suggested_filename()
	_update_file_dialog_filters()
