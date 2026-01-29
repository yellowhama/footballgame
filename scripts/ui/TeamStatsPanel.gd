extends Panel
class_name TeamStatsPanel

@onready var stats_grid: VBoxContainer = $StatsGrid

const STAT_SEPARATION := 20


func set_stats(home_stats: Dictionary, away_stats: Dictionary, lite: bool = false) -> void:
        if not stats_grid:
                return

        _clear_stats()
        _add_stat_row(
                "점유율", _format_percent(home_stats.get("possession", 0)), _format_percent(away_stats.get("possession", 0))
        )
        _add_stat_row("슈팅", str(home_stats.get("shots", 0)), str(away_stats.get("shots", 0)))
        _add_stat_row("유효슈팅", str(home_stats.get("shots_on_target", 0)), str(away_stats.get("shots_on_target", 0)))
        _add_stat_row("xG", _format_decimal(home_stats.get("xg", 0.0)), _format_decimal(away_stats.get("xg", 0.0)))
        _add_stat_row(
                "패스 정확도",
                _format_percent(home_stats.get("pass_accuracy", 0)),
                _format_percent(away_stats.get("pass_accuracy", 0))
        )
        if lite:
                return
        _add_stat_row("패스", str(home_stats.get("passes", 0)), str(away_stats.get("passes", 0)))
        _add_stat_row("코너킥", str(home_stats.get("corners", 0)), str(away_stats.get("corners", 0)))
        _add_stat_row("파울", str(home_stats.get("fouls", 0)), str(away_stats.get("fouls", 0)))

func _clear_stats() -> void:
	for child in stats_grid.get_children():
		child.queue_free()


func _add_stat_row(stat_name: String, home_value: String, away_value: String) -> void:
	var row := HBoxContainer.new()
	row.set("theme_override_constants/separation", STAT_SEPARATION)

	var home_label := _create_value_label(home_value, HORIZONTAL_ALIGNMENT_RIGHT)
	var name_label := _create_name_label(stat_name)
	var away_label := _create_value_label(away_value, HORIZONTAL_ALIGNMENT_LEFT)

	row.add_child(home_label)
	row.add_child(name_label)
	row.add_child(away_label)

	stats_grid.add_child(row)


func _create_value_label(text_value: String, alignment: HorizontalAlignment) -> Label:
	var label := Label.new()
	label.text = text_value
	label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	label.horizontal_alignment = alignment
	return label


func _create_name_label(stat_name: String) -> Label:
	var label := _create_value_label(stat_name, HORIZONTAL_ALIGNMENT_CENTER)
	label.add_theme_color_override("font_color", Color(0.7, 0.7, 0.7))
	return label


func _format_percent(value: Variant) -> String:
	if value is String:
		var trimmed: String = value.strip_edges()
		if trimmed.ends_with("%"):
			return trimmed
		if trimmed == "":
			return "0%"
		var parsed: float = float(trimmed)
		return "%d%%" % int(round(parsed))
	if value is float:
		return "%d%%" % int(round(value))
	return "%d%%" % int(value)


func _format_decimal(value: Variant) -> String:
	if value is float:
		return "%.2f" % value
	return "%.2f" % float(value)
