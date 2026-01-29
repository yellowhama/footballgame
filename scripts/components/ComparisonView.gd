extends PanelContainer
class_name ComparisonView
## Side-by-side record comparison component
## Phase 13: Extended Features - Record Comparison Feature

signal comparison_closed

## Record data
var record_a: Dictionary = {}
var record_b: Dictionary = {}
var comparison_type: String = ""  # "match" or "training"

## UI References
var title_label: Label
var close_button: Button
var comparison_grid: GridContainer
var details_container: VBoxContainer


func _ready():
	_build_ui()


func _build_ui():
	# Main container
	var margin = MarginContainer.new()
	margin.add_theme_constant_override("margin_left", 20)
	margin.add_theme_constant_override("margin_right", 20)
	margin.add_theme_constant_override("margin_top", 16)
	margin.add_theme_constant_override("margin_bottom", 16)
	add_child(margin)

	var vbox = VBoxContainer.new()
	vbox.add_theme_constant_override("separation", 16)
	margin.add_child(vbox)

	# Title bar with close button
	var title_hbox = HBoxContainer.new()
	vbox.add_child(title_hbox)

	title_label = Label.new()
	title_label.text = "기록 비교"
	title_label.add_theme_font_size_override("font_size", 24)
	title_hbox.add_child(title_label)

	var spacer = Control.new()
	spacer.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	title_hbox.add_child(spacer)

	close_button = Button.new()
	close_button.text = "✖ 닫기"
	close_button.custom_minimum_size = Vector2(100, 44)
	close_button.pressed.connect(_on_close_pressed)
	title_hbox.add_child(close_button)

	# Comparison grid (3 columns: Field | Record A | Record B)
	comparison_grid = GridContainer.new()
	comparison_grid.columns = 3
	comparison_grid.add_theme_constant_override("h_separation", 16)
	comparison_grid.add_theme_constant_override("v_separation", 8)
	vbox.add_child(comparison_grid)

	# Details container
	details_container = VBoxContainer.new()
	details_container.add_theme_constant_override("separation", 8)
	vbox.add_child(details_container)


## Set comparison data for match records
func set_match_comparison(match_a: Dictionary, match_b: Dictionary):
	record_a = match_a
	record_b = match_b
	comparison_type = "match"

	_clear_comparison()
	_build_match_comparison()


## Set comparison data for training records
func set_training_comparison(training_a: Dictionary, training_b: Dictionary):
	record_a = training_a
	record_b = training_b
	comparison_type = "training"

	_clear_comparison()
	_build_training_comparison()


func _clear_comparison():
	"""Clear existing comparison data"""
	for child in comparison_grid.get_children():
		child.queue_free()

	for child in details_container.get_children():
		child.queue_free()


func _build_match_comparison():
	"""Build comparison for match records"""
	# Header row
	_add_header_row("항목", "경기 A", "경기 B")

	# Basic info
	_add_comparison_row("날짜", record_a.get("date", ""), record_b.get("date", ""))

	_add_comparison_row(
		"시즌",
		"Year %d Week %d" % [record_a.get("year", 1), record_a.get("week", 1)],
		"Year %d Week %d" % [record_b.get("year", 1), record_b.get("week", 1)]
	)

	_add_comparison_row("상대팀", record_a.get("opponent_name", "Unknown"), record_b.get("opponent_name", "Unknown"))

	# Match result with color coding
	var result_a = record_a.get("result", "draw")
	var result_b = record_b.get("result", "draw")
	_add_comparison_row("결과", result_a, result_b, _get_result_color(result_a), _get_result_color(result_b))

	# Score
	_add_comparison_row(
		"스코어",
		"%d - %d" % [record_a.get("goals_scored", 0), record_a.get("goals_conceded", 0)],
		"%d - %d" % [record_b.get("goals_scored", 0), record_b.get("goals_conceded", 0)]
	)

	# Goal difference with numeric comparison
	var diff_a = record_a.get("goals_scored", 0) - record_a.get("goals_conceded", 0)
	var diff_b = record_b.get("goals_scored", 0) - record_b.get("goals_conceded", 0)
	_add_numeric_comparison_row("득실차", diff_a, diff_b)

	# Match type
	_add_comparison_row("경기 유형", record_a.get("match_type", "friendly"), record_b.get("match_type", "friendly"))

	# Tactic
	_add_comparison_row("전술", record_a.get("tactic_used", "Unknown"), record_b.get("tactic_used", "Unknown"))

	# Opponent rating with numeric comparison
	var rating_a = record_a.get("opponent_rating", 50)
	var rating_b = record_b.get("opponent_rating", 50)
	_add_numeric_comparison_row("상대 레이팅", rating_a, rating_b)

	# Summary
	_add_summary_section()


func _build_training_comparison():
	"""Build comparison for training records"""
	# Header row
	_add_header_row("항목", "훈련 A", "훈련 B")

	# Basic info
	_add_comparison_row("날짜", record_a.get("date", ""), record_b.get("date", ""))

	_add_comparison_row(
		"시즌",
		"Year %d Week %d" % [record_a.get("year", 1), record_a.get("week", 1)],
		"Year %d Week %d" % [record_b.get("year", 1), record_b.get("week", 1)]
	)

	_add_comparison_row("훈련명", record_a.get("training_name", "Unknown"), record_b.get("training_name", "Unknown"))

	_add_comparison_row("훈련 유형", record_a.get("training_type", "unknown"), record_b.get("training_type", "unknown"))

	# Condition
	var cond_before_a = record_a.get("condition_before", 100)
	var cond_after_a = record_a.get("condition_after", 90)
	var cond_before_b = record_b.get("condition_before", 100)
	var cond_after_b = record_b.get("condition_after", 90)

	_add_comparison_row(
		"컨디션 변화", "%.1f%% → %.1f%%" % [cond_before_a, cond_after_a], "%.1f%% → %.1f%%" % [cond_before_b, cond_after_b]
	)

	# Condition cost comparison
	var cost_a = cond_before_a - cond_after_a
	var cost_b = cond_before_b - cond_after_b
	_add_numeric_comparison_row("컨디션 소모", cost_a, cost_b, true)  # Lower is better

	# Effectiveness
	var eff_a = record_a.get("effectiveness_modifier", 1.0)
	var eff_b = record_b.get("effectiveness_modifier", 1.0)
	_add_numeric_comparison_row("훈련 효과", eff_a * 100, eff_b * 100)

	# Attribute changes
	var changes_a = record_a.get("attribute_changes", {})
	var changes_b = record_b.get("attribute_changes", {})

	_add_comparison_row("능력치 변화 수", str(changes_a.size()), str(changes_b.size()))

	# Total attribute gain
	var total_a = 0
	for attr in changes_a:
		total_a += changes_a[attr]

	var total_b = 0
	for attr in changes_b:
		total_b += changes_b[attr]

	_add_numeric_comparison_row("총 능력치 증가", total_a, total_b)

	# Detailed attribute changes
	_add_attribute_changes_detail(changes_a, changes_b)

	# Summary
	_add_summary_section()


func _add_header_row(col1: String, col2: String, col3: String):
	"""Add header row to comparison grid"""
	for text in [col1, col2, col3]:
		var label = Label.new()
		label.text = text
		label.add_theme_font_size_override("font_size", 18)
		label.add_theme_color_override("font_color", Color(1.0, 1.0, 0.6))
		comparison_grid.add_child(label)


func _add_comparison_row(
	field: String, value_a: String, value_b: String, color_a: Color = Color.WHITE, color_b: Color = Color.WHITE
):
	"""Add a comparison row with field name and two values"""
	# Field name
	var field_label = Label.new()
	field_label.text = field
	field_label.add_theme_font_size_override("font_size", 16)
	field_label.add_theme_color_override("font_color", Color(0.8, 0.8, 0.8))
	comparison_grid.add_child(field_label)

	# Value A
	var value_a_label = Label.new()
	value_a_label.text = value_a
	value_a_label.add_theme_font_size_override("font_size", 16)
	value_a_label.add_theme_color_override("font_color", color_a)
	comparison_grid.add_child(value_a_label)

	# Value B
	var value_b_label = Label.new()
	value_b_label.text = value_b
	value_b_label.add_theme_font_size_override("font_size", 16)
	value_b_label.add_theme_color_override("font_color", color_b)
	comparison_grid.add_child(value_b_label)


func _add_numeric_comparison_row(field: String, value_a: float, value_b: float, lower_is_better: bool = false):
	"""Add a comparison row with numeric values and highlighting"""
	var color_a = Color.WHITE
	var color_b = Color.WHITE

	if value_a > value_b:
		color_a = Color(0.5, 1.0, 0.5) if not lower_is_better else Color(1.0, 0.7, 0.7)
		color_b = Color(1.0, 0.7, 0.7) if not lower_is_better else Color(0.5, 1.0, 0.5)
	elif value_b > value_a:
		color_b = Color(0.5, 1.0, 0.5) if not lower_is_better else Color(1.0, 0.7, 0.7)
		color_a = Color(1.0, 0.7, 0.7) if not lower_is_better else Color(0.5, 1.0, 0.5)

	_add_comparison_row(field, str(value_a), str(value_b), color_a, color_b)


func _add_attribute_changes_detail(changes_a: Dictionary, changes_b: Dictionary):
	"""Add detailed attribute changes section"""
	var detail_label = Label.new()
	detail_label.text = "능력치 세부 변화:"
	detail_label.add_theme_font_size_override("font_size", 18)
	detail_label.add_theme_color_override("font_color", Color(1.0, 1.0, 0.6))
	details_container.add_child(detail_label)

	# Combine all attribute names
	var all_attrs = {}
	for attr in changes_a:
		all_attrs[attr] = true
	for attr in changes_b:
		all_attrs[attr] = true

	# Create grid for attributes
	var attr_grid = GridContainer.new()
	attr_grid.columns = 3
	attr_grid.add_theme_constant_override("h_separation", 16)
	attr_grid.add_theme_constant_override("v_separation", 4)
	details_container.add_child(attr_grid)

	for attr in all_attrs.keys():
		var value_a = changes_a.get(attr, 0)
		var value_b = changes_b.get(attr, 0)

		# Attribute name
		var attr_label = Label.new()
		attr_label.text = attr
		attr_grid.add_child(attr_label)

		# Value A
		var val_a_label = Label.new()
		val_a_label.text = "%+d" % value_a
		val_a_label.add_theme_color_override("font_color", Color(0.5, 1.0, 0.5) if value_a > 0 else Color.WHITE)
		attr_grid.add_child(val_a_label)

		# Value B
		var val_b_label = Label.new()
		val_b_label.text = "%+d" % value_b
		val_b_label.add_theme_color_override("font_color", Color(0.5, 1.0, 0.5) if value_b > 0 else Color.WHITE)
		attr_grid.add_child(val_b_label)


func _add_summary_section():
	"""Add summary/verdict section"""
	var summary_label = Label.new()
	summary_label.text = "종합 평가:"
	summary_label.add_theme_font_size_override("font_size", 18)
	summary_label.add_theme_color_override("font_color", Color(1.0, 1.0, 0.6))
	details_container.add_child(summary_label)

	var verdict_text = ""

	if comparison_type == "match":
		var result_a = record_a.get("result", "draw")
		var result_b = record_b.get("result", "draw")
		var diff_a = record_a.get("goals_scored", 0) - record_a.get("goals_conceded", 0)
		var diff_b = record_b.get("goals_scored", 0) - record_b.get("goals_conceded", 0)

		if result_a == "승리" and result_b != "승리":
			verdict_text = "경기 A가 더 좋은 결과입니다 (승리)"
		elif result_b == "승리" and result_a != "승리":
			verdict_text = "경기 B가 더 좋은 결과입니다 (승리)"
		elif diff_a > diff_b:
			verdict_text = "경기 A가 더 좋은 득실차를 기록했습니다 (%+d vs %+d)" % [diff_a, diff_b]
		elif diff_b > diff_a:
			verdict_text = "경기 B가 더 좋은 득실차를 기록했습니다 (%+d vs %+d)" % [diff_b, diff_a]
		else:
			verdict_text = "두 경기의 결과가 유사합니다"

	elif comparison_type == "training":
		var changes_a = record_a.get("attribute_changes", {})
		var changes_b = record_b.get("attribute_changes", {})

		var total_a = 0
		for attr in changes_a:
			total_a += changes_a[attr]

		var total_b = 0
		for attr in changes_b:
			total_b += changes_b[attr]

		if total_a > total_b:
			verdict_text = "훈련 A가 더 많은 능력치 증가를 가져왔습니다 (+%d vs +%d)" % [total_a, total_b]
		elif total_b > total_a:
			verdict_text = "훈련 B가 더 많은 능력치 증가를 가져왔습니다 (+%d vs +%d)" % [total_b, total_a]
		else:
			verdict_text = "두 훈련의 효과가 유사합니다"

	var verdict_label = Label.new()
	verdict_label.text = verdict_text
	verdict_label.add_theme_font_size_override("font_size", 16)
	verdict_label.add_theme_color_override("font_color", Color(0.5, 0.8, 1.0))
	verdict_label.autowrap_mode = TextServer.AUTOWRAP_WORD
	details_container.add_child(verdict_label)


func _get_result_color(result: String) -> Color:
	"""Get color for match result"""
	match result:
		"승리", "win":
			return Color(0.5, 1.0, 0.5)
		"패배", "loss":
			return Color(1.0, 0.5, 0.5)
		_:
			return Color(0.7, 0.7, 0.7)


func _on_close_pressed():
	"""Close button pressed"""
	comparison_closed.emit()
	hide()
