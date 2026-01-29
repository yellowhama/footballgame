extends Control
class_name SeasonEvaluationScreen

@onready var title_label: Label = $VBox/TitleBar/TitleLabel
@onready var class_label: Label = $VBox/Content/SeasonClassLabel
@onready var summary_label: Label = $VBox/Content/SummaryLabel
@onready var value_label: Label = $VBox/Content/MarketValueLabel
@onready var highlights_list: VBoxContainer = $VBox/Content/HighlightsList
@onready var recommendations_list: VBoxContainer = $VBox/Content/RecommendationsList
@onready var close_button: Button = $VBox/BottomBar/CloseButton

var _report: Dictionary = {}


func _ready() -> void:
	if close_button:
		close_button.pressed.connect(_on_close_pressed)


func show_report(report: Dictionary) -> void:
	_report = report.duplicate(true)

	var season: int = int(report.get("season", 1))
	if title_label:
		title_label.text = "Season %d Evaluation" % season

	var summary: Dictionary = report.get("summary", {})
	var class_code: String = str(report.get("player_class", "Average"))
	var mv: int = int(report.get("market_value", 0))

	if class_label:
		class_label.text = _format_class_label(class_code)

	if summary_label:
		summary_label.text = _format_summary_text(summary)

	if value_label:
		value_label.text = _format_market_value_text(mv)

	if highlights_list:
		_populate_list(highlights_list, report.get("highlights", []))

	if recommendations_list:
		_populate_list(recommendations_list, report.get("recommendations", []))


func _format_class_label(class_code: String) -> String:
	match class_code:
		"WK":
			return "올해 등급: WK (World Class)"
		"IK":
			return "올해 등급: IK (International Class)"
		"NK":
			return "올해 등급: NK (National Class)"
		"K":
			return "올해 등급: K (Solid Pro)"
		_:
			return "올해 등급: Average"


func _format_summary_text(summary: Dictionary) -> String:
	var matches: int = int(summary.get("matches", 0))
	var goals: int = int(summary.get("goals", 0))
	var assists: int = int(summary.get("assists", 0))
	var avg_rating: float = float(summary.get("average_rating", 6.0))
	var motm: int = int(summary.get("motm_awards", 0))

	return "경기 %d | 골 %d | 도움 %d\n평균 평점: %.2f | MOM %d회" % [matches, goals, assists, avg_rating, motm]


func _format_market_value_text(mv: int) -> String:
	if mv <= 0:
		return "추정 시장 가치: -"

	# 억 단위로 간단히 변환 (원화 가정)
	var billions: float = float(mv) / 1_000_000_000.0
	return "추정 시장 가치: %.1f억" % billions


func _populate_list(container: VBoxContainer, items: Array) -> void:
	for child in container.get_children():
		child.queue_free()

	for item in items:
		var label := Label.new()
		label.text = str(item)
		label.add_theme_font_size_override("font_size", 16)
		container.add_child(label)


func _on_close_pressed() -> void:
	queue_free()
