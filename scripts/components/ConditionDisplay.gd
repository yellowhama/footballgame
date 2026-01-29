extends Control

# Condition Display Component for v4.0 ConditionSystem
# Shows player condition with Korean names, colors, and detailed information

@onready var condition_label: Label
@onready var condition_bar: ProgressBar
@onready var condition_icon: Label
@onready var condition_details: RichTextLabel

var last_condition_percentage: float = 75.0
var last_condition_level: int = 3


func _ready():
	# Create UI elements if they don't exist
	setup_ui_elements()

	# Connect to condition system
	if ConditionSystem:
		ConditionSystem.condition_changed.connect(_on_condition_changed)
		ConditionSystem.condition_effect_applied.connect(_on_condition_effect_applied)

	# Initial update
	update_display()


func setup_ui_elements():
	"""Setup UI elements if they don't exist"""

	# Create main container
	if not condition_label:
		condition_label = Label.new()
		condition_label.text = "컨디션: 보통"
		condition_label.add_theme_font_size_override("font_size", 18)
		add_child(condition_label)

	if not condition_bar:
		condition_bar = ProgressBar.new()
		condition_bar.min_value = 0
		condition_bar.max_value = 100
		condition_bar.value = 75
		condition_bar.show_percentage = true
		add_child(condition_bar)

	if not condition_icon:
		condition_icon = Label.new()
		condition_icon.text = "⚪"
		condition_icon.add_theme_font_size_override("font_size", 24)
		add_child(condition_icon)

	if not condition_details:
		condition_details = RichTextLabel.new()
		condition_details.fit_content = true
		condition_details.bbcode_enabled = true
		add_child(condition_details)


func _on_condition_changed(new_condition_level, percentage: float):
	"""Handle condition changes from ConditionSystem"""
	last_condition_level = new_condition_level
	last_condition_percentage = percentage
	update_display()


func _on_condition_effect_applied(bonuses: Dictionary):
	"""Handle condition effect applications"""
	update_display()


func update_display():
	"""Update all display elements"""

	var condition_status = get_condition_status()

	# Update label
	if condition_label:
		condition_label.text = "컨디션: %s (%.1f%%)" % [condition_status.name, condition_status.percentage]
		condition_label.modulate = condition_status.color

	# Update progress bar
	if condition_bar:
		condition_bar.value = condition_status.percentage
		# Change bar color based on condition
		var style = StyleBoxFlat.new()
		style.bg_color = condition_status.color
		condition_bar.add_theme_stylebox_override("fill", style)

	# Update icon
	if condition_icon:
		condition_icon.text = get_condition_icon(condition_status.level)
		condition_icon.modulate = condition_status.color

	# Update details
	if condition_details:
		update_condition_details(condition_status)


func get_condition_status() -> Dictionary:
	"""Get current condition status"""
	if ConditionSystem:
		return {
			"level": ConditionSystem.get_condition_level(),
			"percentage": ConditionSystem.get_condition_percentage(),
			"name": ConditionSystem.get_condition_name(),
			"color": ConditionSystem.get_condition_color(),
			"description": ConditionSystem.get_condition_description(),
			"ability_modifier": ConditionSystem.get_ability_modifier(),
			"training_modifier": ConditionSystem.get_training_modifier()
		}
	elif EnhancedPlayerData:
		return EnhancedPlayerData.get_condition_status()
	else:
		return {
			"level": 3,
			"percentage": 75.0,
			"name": "보통",
			"color": Color.WHITE,
			"description": "보통 컨디션",
			"ability_modifier": 1.0,
			"training_modifier": 1.0
		}


func get_condition_icon(level) -> String:
	"""Get condition icon based on level"""
	match level:
		ConditionSystem.ConditionLevel.EXCELLENT:
			return "🔴"  # 절호조
		ConditionSystem.ConditionLevel.GOOD:
			return "🟡"  # 호조
		ConditionSystem.ConditionLevel.AVERAGE:
			return "⚪"  # 보통
		ConditionSystem.ConditionLevel.POOR:
			return "🔵"  # 부진
		ConditionSystem.ConditionLevel.TERRIBLE:
			return "🟣"  # 절부진
		_:
			return "⚪"


func update_condition_details(status: Dictionary):
	"""Update detailed condition information"""

	var details_text = "[center][b]%s[/b][/center]\n" % status.name
	details_text += "컨디션: %.1f%%\n\n" % status.percentage

	# Ability modifier
	var ability_change = (status.ability_modifier - 1.0) * 100
	if ability_change > 0:
		details_text += "[color=green]능력치 보너스: +%.0f%%[/color]\n" % ability_change
	elif ability_change < 0:
		details_text += "[color=red]능력치 페널티: %.0f%%[/color]\n" % ability_change
	else:
		details_text += "능력치 변화: 없음\n"

	# Training modifier
	var training_change = (status.training_modifier - 1.0) * 100
	if training_change > 0:
		details_text += "[color=green]훈련 효과 보너스: +%.0f%%[/color]\n" % training_change
	elif training_change < 0:
		details_text += "[color=red]훈련 효과 페널티: %.0f%%[/color]\n" % training_change
	else:
		details_text += "훈련 효과 변화: 없음\n"

	# Condition advice
	details_text += "\n" + get_condition_advice(status.level)

	condition_details.text = details_text


func get_condition_advice(level) -> String:
	"""Get advice based on condition level"""
	match level:
		ConditionSystem.ConditionLevel.EXCELLENT:
			return "[color=green][b]완벽한 컨디션![/b] 고강도 훈련을 해도 좋습니다.[/color]"
		ConditionSystem.ConditionLevel.GOOD:
			return "[color=yellow][b]좋은 컨디션![/b] 적극적으로 훈련하세요.[/color]"
		ConditionSystem.ConditionLevel.AVERAGE:
			return "[color=white]보통 컨디션입니다. 적당한 훈련이 좋겠어요.[/color]"
		ConditionSystem.ConditionLevel.POOR:
			return "[color=orange][b]컨디션이 나쁩니다.[/b] 가벼운 훈련이나 휴식을 고려하세요.[/color]"
		ConditionSystem.ConditionLevel.TERRIBLE:
			return "[color=red][b]매우 나쁜 컨디션![/b] 반드시 휴식이 필요합니다.[/color]"
		_:
			return "컨디션을 확인할 수 없습니다."


# Quick condition check methods for other UI elements
func get_condition_text() -> String:
	"""Get simple condition text for embedding in other UI"""
	var status = get_condition_status()
	return "%s %s (%.1f%%)" % [get_condition_icon(status.level), status.name, status.percentage]


func get_condition_color() -> Color:
	"""Get condition color for UI theming"""
	var status = get_condition_status()
	return status.color


func is_condition_good() -> bool:
	"""Check if condition is good for training"""
	var status = get_condition_status()
	return status.level >= ConditionSystem.ConditionLevel.GOOD


func is_condition_bad() -> bool:
	"""Check if condition is bad and needs rest"""
	var status = get_condition_status()
	return status.level <= ConditionSystem.ConditionLevel.POOR


# Daily condition changes display
func show_daily_summary():
	"""Show daily condition changes summary"""
	if not ConditionSystem:
		return

	var summary = ConditionSystem.get_daily_summary()
	if summary.factors.size() == 0:
		return

	var popup = AcceptDialog.new()
	popup.title = "오늘의 컨디션 변화"

	var text = "오늘의 컨디션 변화:\n\n"
	for factor in summary.factors:
		var change_text = "+%.1f" % factor.value if factor.value > 0 else "%.1f" % factor.value
		text += "• %s: %s (%s)\n" % [factor.reason, change_text, factor.type]

	text += "\n총 변화: %+.1f" % summary.total_change
	text += "\n현재 컨디션: %s (%.1f%%)" % [summary.final_level, summary.final_condition]

	var label = RichTextLabel.new()
	label.text = text
	label.fit_content = true
	label.custom_minimum_size = Vector2(400, 200)
	popup.add_child(label)

	get_tree().root.add_child(popup)
	popup.popup_centered()


# Animation and effects
func animate_condition_change(old_value: float, new_value: float):
	"""Animate condition change"""
	var tween = create_tween()

	# Animate progress bar
	if condition_bar:
		tween.tween_property(condition_bar, "value", new_value, 0.5)

	# Flash effect for significant changes
	var change = abs(new_value - old_value)
	if change > 10.0:
		var flash_color = Color.GREEN if new_value > old_value else Color.RED
		modulate = flash_color
		tween.tween_property(self, "modulate", Color.WHITE, 0.3)


# Testing and debug methods
func test_condition_levels():
	"""Test all condition levels for debugging"""
	var test_values = [15, 35, 50, 70, 85, 98]
	var test_names = ["절부진", "부진", "보통", "보통+", "호조", "절호조"]

	for i in range(test_values.size()):
		if ConditionSystem:
			ConditionSystem.set_condition_for_testing(test_values[i])
		await get_tree().create_timer(1.0).timeout


func simulate_daily_changes():
	"""Simulate daily condition changes for testing"""
	if ConditionSystem:
		ConditionSystem.simulate_week_condition_changes()
		show_daily_summary()
