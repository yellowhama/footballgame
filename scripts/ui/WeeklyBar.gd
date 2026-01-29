extends Control
# 주간 정보 표시 바


func _ready():
	pass


func update_week_info(week: int, year: int):
	# 주간 정보 업데이트
	var week_label = get_node_or_null("Content/WeekInfo/WeekLabel")
	if week_label:
		week_label.text = "Week %d, %d" % [week, year]


func update_condition(condition: int):
	# 컨디션 정보 업데이트
	var condition_label = get_node_or_null("Content/ConditionInfo/ConditionLabel")
	if condition_label:
		var condition_text = ""
		match condition:
			5:
				condition_text = "Perfect"
			4:
				condition_text = "Good"
			3:
				condition_text = "Normal"
			2:
				condition_text = "Poor"
			1:
				condition_text = "Terrible"
			_:
				condition_text = "Unknown"
		condition_label.text = condition_text
