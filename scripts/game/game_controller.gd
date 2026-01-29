extends Node2D

var current_week: int = 1
var current_year: int = 1


func _ready():
	print("Game Scene loaded!")
	_update_week_display()

	# GameManager가 있으면 연결
	if has_node("/root/GameManager"):
		var game_manager = get_node("/root/GameManager")
		current_week = game_manager.get_current_week()
		current_year = game_manager.get_current_year()


func _update_week_display():
	var week_label = $UI/WeeklyView/Header/WeekLabel
	if week_label:
		week_label.text = "Week %d - Year %d" % [current_week, current_year]


func _on_training_pressed():
	print("Opening training...")
	# 훈련 화면으로 전환
	if has_node("/root/SceneLoader"):
		get_node("/root/SceneLoader").load_scene("res://scenes/training/TrainingScreenImproved_Responsive.tscn")
	else:
		get_tree().change_scene_to_file("res://scenes/training/TrainingScreenImproved_Responsive.tscn")


func _on_match_pressed():
	print("Starting match simulation...")
	# Open-Football 엔진을 사용한 경기 시뮬레이션
	_simulate_match()


func _on_stats_pressed():
	print("Opening stats view...")
	# 스탯 화면으로 전환
	get_tree().change_scene_to_file("res://scenes/StatusScreenImproved.tscn")


func _on_next_week_pressed():
	print("Advancing to next week...")
	_advance_week()


func _advance_week():
	current_week += 1
	if current_week > 52:
		current_week = 1
		current_year += 1
		if current_year > 3:
			print("Game completed! Contract decision time!")
			_show_graduation_screen()
			return

	# GameManager 업데이트
	if has_node("/root/GameManager"):
		get_node("/root/GameManager").advance_week()

	# 주간 이벤트 체크
	if has_node("/root/WeeklyEventSystem"):
		get_node("/root/WeeklyEventSystem").check_weekly_events()

	_update_week_display()


func _simulate_match():
	# TODO: Open-Football 엔진 통합
	print("Match simulation would run here...")

	# 임시 결과
	var match_result = {"home_score": randi() % 5, "away_score": randi() % 5, "player_performance": randf() * 10}

	print("Match Result: %d - %d" % [match_result.home_score, match_result.away_score])
	print("Your performance: %.1f" % match_result.player_performance)


func _show_graduation_screen():
	print("Congratulations! You've completed the academy!")
	# TODO: 졸업 화면으로 전환
