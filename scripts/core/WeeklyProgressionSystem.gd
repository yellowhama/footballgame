extends Node

# Weekly Progression System
# 주간 단위로 게임을 진행하고 모든 시스템을 통합 관리

signal week_advanced(new_week: int, new_year: int)
signal week_events_triggered(events: Array)
signal season_ended(year: int)
signal graduation_reached

# 현재 게임 진행 상태
var current_week: int = 1
var current_year: int = 1
var total_weeks: int = 0

# 주간 이벤트 시스템
var weekly_events: Array = []


func _ready():
	print("[WeeklyProgressionSystem] Initializing...")
	load_progress()


func get_current_progress() -> Dictionary:
	"""현재 진행 상태 반환"""
	return {"week": current_week, "year": current_year, "total_weeks": total_weeks}


func advance_week() -> Dictionary:
	"""주차 진행"""
	current_week += 1
	total_weeks += 1

	# 52주가 지나면 다음 해
	if current_week > 52:
		current_year += 1
		current_week = 1
		season_ended.emit(current_year - 1)

	# 3년이 지나면 졸업
	if current_year > 3:
		graduation_reached.emit()

	# 주간 이벤트 처리
	var events = process_weekly_events()
	if events.size() > 0:
		week_events_triggered.emit(events)

	# 진행 상태 저장
	save_progress()

	# 신호 발생
	week_advanced.emit(current_week, current_year)

	return {"week": current_week, "year": current_year, "events": events}


func process_weekly_events() -> Array:
	"""주간 이벤트 처리"""
	var events = []

	# 랜덤 이벤트 (10% 확률)
	if randf() < 0.1:
		events.append({"type": "random_event", "description": "특별한 일이 발생했습니다!", "week": current_week})

	# 특정 주차 이벤트
	match current_week:
		1:
			events.append({"type": "season_start", "description": "새로운 시즌이 시작됩니다!", "week": current_week})
		52:
			events.append({"type": "season_end", "description": "시즌이 끝났습니다!", "week": current_week})

	return events


func get_weekly_summary() -> Dictionary:
	"""주간 요약 정보 반환"""
	return {
		"current_week": current_week,
		"current_year": current_year,
		"total_weeks": total_weeks,
		"weeks_remaining": (3 * 52) - total_weeks,
		"progress_percentage": float(total_weeks) / (3 * 52) * 100.0
	}


func save_progress():
	"""진행 상태 저장"""
	var save_data = {"current_week": current_week, "current_year": current_year, "total_weeks": total_weeks}

	var file = FileAccess.open("user://weekly_progress.save", FileAccess.WRITE)
	if file:
		file.store_string(JSON.stringify(save_data))
		file.close()


func load_progress():
	"""진행 상태 로드"""
	var file = FileAccess.open("user://weekly_progress.save", FileAccess.READ)
	if file:
		var json = JSON.new()
		var parse_result = json.parse(file.get_as_text())
		file.close()

		if parse_result == OK:
			var data = json.data
			current_week = data.get("current_week", 1)
			current_year = data.get("current_year", 1)
			total_weeks = data.get("total_weeks", 0)


func reset_progress():
	"""진행 상태 초기화"""
	current_week = 1
	current_year = 1
	total_weeks = 0
	save_progress()
