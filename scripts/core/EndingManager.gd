extends Node
# EndingManager - 5가지 엔딩 시스템 관리

signal ending_triggered(ending_type: String)

# 엔딩 타입 정의
enum EndingType { PRO_SUPERSTAR, OVERSEAS_STUDY, UNIVERSITY_ACE, COACH_PATH, HIDDEN_LEGEND }  # 프로 슈퍼스타 엔딩  # 해외 유학 엔딩  # 대학 에이스 엔딩  # 지도자의 길 엔딩  # 히든 레전드 엔딩

# 엔딩 조건 데이터
var ending_conditions = {
	EndingType.PRO_SUPERSTAR:
	{
		"name": "프로 슈퍼스타",
		"description": "최고의 프로 선수가 되어 월드컵에서 활약한다",
		"requirements": {"min_ca": 150, "min_matches": 50, "min_goals": 30, "special_ability": "슈퍼스타"},
		"icon": "⭐",
		"rarity": "S"
	},
	EndingType.OVERSEAS_STUDY:
	{
		"name": "해외 유학",
		"description": "유럽 명문 클럽의 스카우트 제안을 받아 해외로 진출한다",
		"requirements": {"min_ca": 120, "min_technical": 80, "min_reputation": 70, "language_skill": true},
		"icon": "✈️",
		"rarity": "A"
	},
	EndingType.UNIVERSITY_ACE:
	{
		"name": "대학 에이스",
		"description": "명문 대학 축구부의 에이스로 활약하며 학업과 운동을 병행한다",
		"requirements": {"min_ca": 100, "min_academic": 75, "balanced_stats": true, "leadership": 60},
		"icon": "🎓",
		"rarity": "B"
	},
	EndingType.COACH_PATH:
	{
		"name": "지도자의 길",
		"description": "선수 생활을 마치고 젊은 후배들을 지도하는 코치가 된다",
		"requirements": {"min_ca": 80, "min_mental": 80, "coaching_experience": true, "mentor_relationships": 3},
		"icon": "👨‍🏫",
		"rarity": "B"
	},
	EndingType.HIDDEN_LEGEND:
	{
		"name": "히든 레전드",
		"description": "숨겨진 잠재력을 모두 개화시켜 전설적인 선수가 된다",
		"requirements":
		{
			"max_potential_reached": true,
			"all_special_abilities": true,
			"perfect_hexagon": true,
			"hidden_events_completed": 5
		},
		"icon": "👑",
		"rarity": "SS"
	}
}

# 현재 게임 상태
var current_game_state = {}
var player_stats = {}
var achievements = []
var story_flags = {}


func _ready():
	print("[EndingManager] Initializing ending system...")
	_connect_game_events()


func _connect_game_events():
	"""게임 이벤트와 연결"""
	# EventBus와 연결 (만약 있다면)
	if has_node("/root/EventBus"):
		var event_bus = get_node("/root/EventBus")
		# 게임 종료 시점 체크
		event_bus.connect("season_ended", _check_endings)
		event_bus.connect("graduation_approaching", _check_endings)
		event_bus.connect("special_event_completed", _on_special_event_completed)


func update_player_stats(stats: Dictionary):
	"""플레이어 스탯 업데이트"""
	player_stats = stats
	print("[EndingManager] Player stats updated: CA=%d, PA=%d" % [stats.get("ca", 0), stats.get("pa", 0)])


func update_game_state(state: Dictionary):
	"""게임 상태 업데이트"""
	current_game_state = state
	print("[EndingManager] Game state updated: Week %d, Season %d" % [state.get("week", 0), state.get("season", 0)])


func add_achievement(achievement: String):
	"""업적 추가"""
	if achievement not in achievements:
		achievements.append(achievement)
		print("[EndingManager] Achievement unlocked: %s" % achievement)


func set_story_flag(flag: String, value: bool = true):
	"""스토리 플래그 설정"""
	story_flags[flag] = value
	print("[EndingManager] Story flag set: %s = %s" % [flag, value])


func _check_endings():
	"""엔딩 조건 체크"""
	print("[EndingManager] Checking ending conditions...")

	var available_endings = []

	# 각 엔딩 조건 체크
	for ending_type in EndingType.values():
		if _check_ending_condition(ending_type):
			available_endings.append(ending_type)

	# 엔딩 우선순위 정렬 (레어도 순)
	available_endings.sort_custom(_sort_endings_by_priority)

	if available_endings.size() > 0:
		var best_ending = available_endings[0]
		_trigger_ending(best_ending)
	else:
		print("[EndingManager] No ending conditions met")


func _check_ending_condition(ending_type: int) -> bool:
	"""특정 엔딩 조건 체크"""
	var conditions = ending_conditions[ending_type]["requirements"]

	match ending_type:
		EndingType.PRO_SUPERSTAR:
			return _check_pro_superstar_conditions(conditions)
		EndingType.OVERSEAS_STUDY:
			return _check_overseas_study_conditions(conditions)
		EndingType.UNIVERSITY_ACE:
			return _check_university_ace_conditions(conditions)
		EndingType.COACH_PATH:
			return _check_coach_path_conditions(conditions)
		EndingType.HIDDEN_LEGEND:
			return _check_hidden_legend_conditions(conditions)

	return false


func _check_pro_superstar_conditions(conditions: Dictionary) -> bool:
	"""프로 슈퍼스타 엔딩 조건 체크"""
	var ca = player_stats.get("ca", 0)
	var matches = current_game_state.get("total_matches", 0)
	var goals = current_game_state.get("total_goals", 0)
	var has_special = "슈퍼스타" in achievements

	return (
		ca >= conditions.min_ca and matches >= conditions.min_matches and goals >= conditions.min_goals and has_special
	)


func _check_overseas_study_conditions(conditions: Dictionary) -> bool:
	"""해외 유학 엔딩 조건 체크"""
	var ca = player_stats.get("ca", 0)
	var technical = player_stats.get("technical_average", 0)
	var reputation = current_game_state.get("reputation", 0)
	var language = story_flags.get("language_skill_learned", false)

	return (
		ca >= conditions.min_ca
		and technical >= conditions.min_technical
		and reputation >= conditions.min_reputation
		and language
	)


func _check_university_ace_conditions(conditions: Dictionary) -> bool:
	"""대학 에이스 엔딩 조건 체크"""
	var ca = player_stats.get("ca", 0)
	var academic = current_game_state.get("academic_score", 0)
	var leadership = player_stats.get("leadership", 0)
	var balanced = _check_balanced_stats()

	return (
		ca >= conditions.min_ca
		and academic >= conditions.min_academic
		and leadership >= conditions.leadership
		and balanced
	)


func _check_coach_path_conditions(conditions: Dictionary) -> bool:
	"""지도자의 길 엔딩 조건 체크"""
	var ca = player_stats.get("ca", 0)
	var mental = player_stats.get("mental_average", 0)
	var coaching_exp = story_flags.get("coaching_experience", false)
	var mentors = current_game_state.get("mentor_count", 0)

	return (
		ca >= conditions.min_ca
		and mental >= conditions.min_mental
		and coaching_exp
		and mentors >= conditions.mentor_relationships
	)


func _check_hidden_legend_conditions(conditions: Dictionary) -> bool:
	"""히든 레전드 엔딩 조건 체크"""
	var potential_maxed = player_stats.get("ca", 0) >= player_stats.get("pa", 0)
	var all_abilities = _check_all_special_abilities()
	var perfect_hex = _check_perfect_hexagon()
	var hidden_events = story_flags.get("hidden_events_completed", 0)

	return potential_maxed and all_abilities and perfect_hex and hidden_events >= conditions.hidden_events_completed


func _check_balanced_stats() -> bool:
	"""균형잡힌 스탯 체크"""
	var stats = ["technical", "physical", "mental", "pace", "power", "defending"]
	var min_threshold = 70

	for stat in stats:
		if player_stats.get(stat + "_average", 0) < min_threshold:
			return false
	return true


func _check_all_special_abilities() -> bool:
	"""모든 특수능력 보유 체크"""
	var required_abilities = ["슈퍼스타", "캡틴", "완벽주의자", "천재", "멘탈갑"]
	for ability in required_abilities:
		if ability not in achievements:
			return false
	return true


func _check_perfect_hexagon() -> bool:
	"""완벽한 6각형 체크"""
	var hexagon_stats = ["technical", "pace", "power", "defending", "mental", "physical"]
	var min_perfect = 90

	for stat in hexagon_stats:
		if player_stats.get(stat + "_average", 0) < min_perfect:
			return false
	return true


func _sort_endings_by_priority(a: int, b: int) -> bool:
	"""엔딩 우선순위 정렬 (레어도 순)"""
	var rarity_order = {"SS": 0, "S": 1, "A": 2, "B": 3, "C": 4}
	var rarity_a = ending_conditions[a]["rarity"]
	var rarity_b = ending_conditions[b]["rarity"]
	return rarity_order[rarity_a] < rarity_order[rarity_b]


func _trigger_ending(ending_type: int):
	"""엔딩 트리거"""
	var ending_data = ending_conditions[ending_type]
	print("[EndingManager] Triggering ending: %s" % ending_data.name)

	# 엔딩 시그널 발송
	ending_triggered.emit(ending_data.name)

	# 엔딩 화면으로 전환
	_show_ending_screen(ending_type, ending_data)


func _show_ending_screen(ending_type: int, ending_data: Dictionary):
	"""엔딩 화면 표시"""
	# EndingScreen 씬으로 전환
	get_tree().change_scene_to_file("res://scenes/EndingScreen.tscn")

	# 엔딩 데이터를 전역으로 저장해서 EndingScreen에서 사용할 수 있도록
	if has_node("/root/GameData"):
		var game_data = get_node("/root/GameData")
		game_data.ending_type = ending_type
		game_data.ending_data = ending_data
		game_data.player_final_stats = player_stats
		game_data.final_achievements = achievements


func _on_special_event_completed(event_name: String):
	"""특수 이벤트 완료 처리"""
	print("[EndingManager] Special event completed: %s" % event_name)

	# 히든 이벤트 카운터 증가
	if event_name.begins_with("hidden_"):
		var current_count = story_flags.get("hidden_events_completed", 0)
		story_flags["hidden_events_completed"] = current_count + 1

	# 특정 이벤트에 따른 플래그 설정
	match event_name:
		"language_course_completed":
			set_story_flag("language_skill_learned", true)
		"coaching_workshop_attended":
			set_story_flag("coaching_experience", true)
		"mentor_relationship_formed":
			var mentor_count = current_game_state.get("mentor_count", 0)
			current_game_state["mentor_count"] = mentor_count + 1


# 외부에서 호출할 수 있는 디버그 함수들
func debug_trigger_ending(ending_name: String):
	"""디버그용 엔딩 강제 트리거"""
	for ending_type in EndingType.values():
		if ending_conditions[ending_type]["name"] == ending_name:
			_trigger_ending(ending_type)
			return
	print("[EndingManager] Debug: Ending not found: %s" % ending_name)


func debug_show_ending_status():
	"""디버그용 엔딩 상태 표시"""
	print("[EndingManager] Current ending status:")
	for ending_type in EndingType.values():
		var ending_data = ending_conditions[ending_type]
		var meets_condition = _check_ending_condition(ending_type)
		print("  %s %s: %s" % [ending_data.icon, ending_data.name, "✅" if meets_condition else "❌"])


func get_ending_progress() -> Dictionary:
	"""엔딩 진행 상황 반환"""
	var progress = {}
	for ending_type in EndingType.values():
		var ending_data = ending_conditions[ending_type]
		progress[ending_data.name] = {
			"meets_condition": _check_ending_condition(ending_type),
			"icon": ending_data.icon,
			"rarity": ending_data.rarity,
			"description": ending_data.description
		}
	return progress
