extends PanelContainer
class_name GrowthResultPanel
##
## GrowthResultPanel
##
## Phase 5.6: 경기 후 Hero Time 스탯 성장 결과 표시 패널
##
## 기능:
## - 성장한 스탯 목록 표시
## - 증가량 애니메이션 (+1, +2, +3)
## - 하이라이트 이펙트
## - 총 XP 획득량 표시
##

signal panel_closed

@onready var title_label: Label = $VBox/TitleLabel
@onready var total_xp_label: Label = $VBox/TotalXPLabel
@onready var stats_container: VBoxContainer = $VBox/StatsScroll/StatsContainer
@onready var close_button: Button = $VBox/CloseButton

## Colors for stat gain display
const COLOR_GAIN_1: Color = Color(0.3, 0.9, 0.3)  # Green for +1
const COLOR_GAIN_2: Color = Color(0.9, 0.9, 0.3)  # Yellow for +2
const COLOR_GAIN_3: Color = Color(0.9, 0.5, 0.3)  # Orange for +3

## Animation timing
const STAT_REVEAL_DELAY: float = 0.3
const STAT_FADE_DURATION: float = 0.4


func _ready() -> void:
	visible = false
	if close_button:
		close_button.pressed.connect(_on_close_pressed)


func show_growth_result(growth: Dictionary) -> void:
	"""
	성장 결과 표시
	@param growth: HeroMatchGrowth Dictionary
		{
			"stat_gains": { "passing": 1, "dribbling": 2 },
			"xp_overflow": { "passing": 5.3 },
			"total_xp_earned": 45.5,
			"highlight_gains": [["passing", 1], ["dribbling", 2]]
		}
	"""
	# Clear previous entries
	for child in stats_container.get_children():
		child.queue_free()

	# Wait a frame for queue_free to process
	await get_tree().process_frame

	# Update title
	if title_label:
		title_label.text = "STAT GROWTH"

	# Update total XP
	var total_xp: float = growth.get("total_xp_earned", 0.0)
	if total_xp_label:
		total_xp_label.text = "Total XP: %.1f" % total_xp

	# Get highlight gains for display
	var highlight_gains: Array = growth.get("highlight_gains", [])
	var stat_gains: Dictionary = growth.get("stat_gains", {})

	# If no highlight_gains, build from stat_gains
	if highlight_gains.is_empty() and not stat_gains.is_empty():
		for stat_name in stat_gains:
			var gain: int = int(stat_gains[stat_name])
			if gain > 0:
				highlight_gains.append([stat_name, gain])

	# Check if there are any gains to display
	if highlight_gains.is_empty():
		_add_no_growth_message()
		visible = true
		return

	# Add stat entries with animation
	visible = true

	var delay: float = 0.0
	for item in highlight_gains:
		if not (item is Array) or item.size() < 2:
			continue

		var stat_name: String = str(item[0])
		var gain: int = int(item[1])

		await get_tree().create_timer(delay).timeout
		_add_stat_entry(stat_name, gain)
		delay += STAT_REVEAL_DELAY

	# Focus close button
	if close_button:
		close_button.grab_focus()


func _add_stat_entry(stat_name: String, gain: int) -> void:
	"""Add a single stat entry with animation"""
	var container := HBoxContainer.new()
	container.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	# Stat name label
	var name_label := Label.new()
	name_label.text = _format_stat_name(stat_name)
	name_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	name_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_LEFT

	# Gain label
	var gain_label := Label.new()
	gain_label.text = "+%d" % gain
	gain_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	gain_label.add_theme_font_size_override("font_size", 20)

	# Color based on gain amount
	var color: Color = COLOR_GAIN_1
	match gain:
		2:
			color = COLOR_GAIN_2
		3:
			color = COLOR_GAIN_3
	gain_label.add_theme_color_override("font_color", color)

	container.add_child(name_label)
	container.add_child(gain_label)
	stats_container.add_child(container)

	# Fade-in animation
	container.modulate.a = 0.0
	var tween := create_tween()
	tween.tween_property(container, "modulate:a", 1.0, STAT_FADE_DURATION)

	# Scale pop effect on gain label
	gain_label.pivot_offset = gain_label.size / 2
	gain_label.scale = Vector2(0.5, 0.5)
	(
		tween
		. parallel()
		. tween_property(gain_label, "scale", Vector2(1.0, 1.0), STAT_FADE_DURATION)
		. set_ease(Tween.EASE_OUT)
		. set_trans(Tween.TRANS_BACK)
	)


func _add_no_growth_message() -> void:
	"""Add a message when no stats grew"""
	var label := Label.new()
	label.text = "No stat growth this match.\nKeep practicing in Hero Time!"
	label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.add_theme_color_override("font_color", Color(0.7, 0.7, 0.7))
	stats_container.add_child(label)


func _format_stat_name(stat_name: String) -> String:
	"""Format stat name for display (snake_case → Title Case)"""
	# Korean translations for common stats
	var translations: Dictionary = {
		"passing": "패스",
		"dribbling": "드리블",
		"finishing": "골 결정력",
		"long_shots": "중거리 슛",
		"vision": "시야",
		"composure": "침착성",
		"decisions": "판단력",
		"anticipation": "예측력",
		"tackling": "태클",
		"marking": "마킹",
		"positioning": "위치 선정",
		"strength": "피지컬",
		"agility": "민첩성",
		"pace": "스피드",
		"acceleration": "가속력",
		"stamina": "지구력",
		"technique": "기술",
		"first_touch": "퍼스트 터치",
		"crossing": "크로스",
		"heading": "헤딩",
		"free_kicks": "프리킥",
		"determination": "투지",
		"concentration": "집중력",
		"teamwork": "팀워크",
		"work_rate": "활동량",
		"flair": "독창성",
		"off_the_ball": "오프더볼",
		"leadership": "리더십",
		"bravery": "용감함",
		"aggression": "적극성"
	}

	if translations.has(stat_name):
		return translations[stat_name]

	# Fallback: convert snake_case to Title Case
	var parts := stat_name.split("_")
	var result := ""
	for part in parts:
		if result != "":
			result += " "
		result += part.capitalize()
	return result


func _on_close_pressed() -> void:
	visible = false
	panel_closed.emit()
