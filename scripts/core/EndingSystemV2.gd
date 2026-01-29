# scripts/core/EndingSystemV2.gd
# 5가지 엔딩 시스템 - 완전 새로운 구현
extends Node

enum EndingType {NONE = 0, PRO_SUPERSTAR = 1, OVERSEAS_STUDY = 2, UNIVERSITY_ACE = 3, COACH_PATH = 4, HIDDEN_LEGEND = 5}  # 프로 슈퍼스타  # 해외 유학  # 대학 에이스  # 지도자의 길  # 히든 레전드

# 엔딩 조건 데이터
var ending_conditions = {
	EndingType.PRO_SUPERSTAR: {"ca_min": 170, "mvp_count": 2, "goals_season": 15, "reputation": 90, "physical": 85},
	EndingType.OVERSEAS_STUDY:
	{"academic_score": 95, "english_skill": 90, "international_exp": true, "ca_min": 160, "adaptability": 80},
	EndingType.UNIVERSITY_ACE:
	{"ca_min": 150, "academic_score": 85, "leadership": 90, "team_chemistry": 85, "loyalty": 80},
	EndingType.COACH_PATH:
	{"leadership": 95, "tactical_knowledge": 90, "coaching_exp": true, "mentoring_count": 3, "team_captain": true},
	EndingType.HIDDEN_LEGEND: {"secret_events": 5, "perfect_matches": 3, "hidden_stats": 200, "legendary_moment": true}
}

# 현재 달성한 조건들 추적
var current_achievements = {
	"mvp_count": 0,
	"goals_season": 0,
	"international_exp": false,
	"coaching_exp": false,
	"mentoring_count": 0,
	"team_captain": false,
	"secret_events": 0,
	"perfect_matches": 0,
	"legendary_moment": false
}

signal ending_triggered(ending_type: EndingType)
signal ending_condition_met(condition: String, value: Variant)


func _ready():
	# EventBus 연결
	EventBus.subscribe("match_completed", _on_match_completed)
	EventBus.subscribe("season_completed", _on_season_completed)
	EventBus.subscribe("special_event", _on_special_event)
	EventBus.subscribe("academic_grade", _on_academic_grade)


# 메인 엔딩 체크 함수
func check_all_endings() -> EndingType:
	var player_data = PlayerData  # 또는 EnhancedPlayerData

	# 우선순위 순서로 체크 (히든 > 프로 > 해외 > 대학 > 코치)
	if check_hidden_legend_ending(player_data):
		return EndingType.HIDDEN_LEGEND

	if check_pro_superstar_ending(player_data):
		return EndingType.PRO_SUPERSTAR

	if check_overseas_study_ending(player_data):
		return EndingType.OVERSEAS_STUDY

	if check_university_ace_ending(player_data):
		return EndingType.UNIVERSITY_ACE

	if check_coach_path_ending(player_data):
		return EndingType.COACH_PATH

	return EndingType.NONE


# 개별 엔딩 체크 함수들
func check_pro_superstar_ending(player_data) -> bool:
	var conditions = ending_conditions[EndingType.PRO_SUPERSTAR]
	var ca = _calculate_player_ca(player_data)

	return (
		ca >= conditions.ca_min
		and current_achievements.mvp_count >= conditions.mvp_count
		and current_achievements.goals_season >= conditions.goals_season
		and player_data.reputation >= conditions.reputation
		and player_data.get_stat("physical") >= conditions.physical
	)


func check_overseas_study_ending(player_data) -> bool:
	var conditions = ending_conditions[EndingType.OVERSEAS_STUDY]
	var ca = _calculate_player_ca(player_data)

	return (
		player_data.academic_score >= conditions.academic_score
		and player_data.get_stat("english_skill") >= conditions.english_skill
		and current_achievements.international_exp == conditions.international_exp
		and ca >= conditions.ca_min
		and player_data.get_stat("adaptability") >= conditions.adaptability
	)


func check_university_ace_ending(player_data) -> bool:
	var conditions = ending_conditions[EndingType.UNIVERSITY_ACE]
	var ca = _calculate_player_ca(player_data)

	return (
		ca >= conditions.ca_min
		and player_data.academic_score >= conditions.academic_score
		and player_data.get_stat("leadership") >= conditions.leadership
		and player_data.team_chemistry >= conditions.team_chemistry
		and player_data.get_stat("loyalty") >= conditions.loyalty
	)


func check_coach_path_ending(player_data) -> bool:
	var conditions = ending_conditions[EndingType.COACH_PATH]

	return (
		player_data.get_stat("leadership") >= conditions.leadership
		and player_data.get_stat("tactical_knowledge") >= conditions.tactical_knowledge
		and current_achievements.coaching_exp == conditions.coaching_exp
		and current_achievements.mentoring_count >= conditions.mentoring_count
		and current_achievements.team_captain == conditions.team_captain
	)


func _calculate_player_ca(player_data) -> int:
	if not player_data:
		return 0
	if player_data.has_method("get_overall_rating"):
		return int(player_data.get_overall_rating())
	if player_data.has("overall_rating"):
		return int(player_data["overall_rating"])
	if player_data.has("overall"):
		return int(player_data["overall"])
	return 0


func check_hidden_legend_ending(player_data) -> bool:
	var conditions = ending_conditions[EndingType.HIDDEN_LEGEND]
	var hidden_stats_total = (
		player_data.get_stat("determination")
		+ player_data.get_stat("mental_strength")
		+ player_data.get_stat("passion")
	)

	return (
		current_achievements.secret_events >= conditions.secret_events
		and current_achievements.perfect_matches >= conditions.perfect_matches
		and hidden_stats_total >= conditions.hidden_stats
		and current_achievements.legendary_moment == conditions.legendary_moment
	)


# 엔딩 실행
func trigger_ending(ending_type: EndingType):
	print("엔딩 발생: ", get_ending_name(ending_type))

	# 엔딩별 전용 씬으로 전환 (Maaacks SceneLoader 사용)
	match ending_type:
		EndingType.PRO_SUPERSTAR:
			SceneLoader.load_scene("res://scenes/endings/ProSuperstarEnding.tscn")
		EndingType.OVERSEAS_STUDY:
			SceneLoader.load_scene("res://scenes/endings/OverseasStudyEnding.tscn")
		EndingType.UNIVERSITY_ACE:
			SceneLoader.load_scene("res://scenes/endings/UniversityAceEnding.tscn")
		EndingType.COACH_PATH:
			SceneLoader.load_scene("res://scenes/endings/CoachPathEnding.tscn")
		EndingType.HIDDEN_LEGEND:
			SceneLoader.load_scene("res://scenes/endings/HiddenLegendEnding.tscn")

	ending_triggered.emit(ending_type)


# 유틸리티 함수들
func get_ending_name(ending_type: EndingType) -> String:
	match ending_type:
		EndingType.PRO_SUPERSTAR:
			return "프로 슈퍼스타"
		EndingType.OVERSEAS_STUDY:
			return "해외 유학"
		EndingType.UNIVERSITY_ACE:
			return "대학 에이스"
		EndingType.COACH_PATH:
			return "지도자의 길"
		EndingType.HIDDEN_LEGEND:
			return "히든 레전드"
		_:
			return "일반 졸업"


func get_ending_description(ending_type: EndingType) -> String:
	match ending_type:
		EndingType.PRO_SUPERSTAR:
			return "뛰어난 실력으로 프로 무대에서 슈퍼스타가 되었습니다!"
		EndingType.OVERSEAS_STUDY:
			return "학업과 축구 실력을 인정받아 해외 유학의 기회를 얻었습니다!"
		EndingType.UNIVERSITY_ACE:
			return "균형잡힌 실력으로 명문대학의 에이스가 되었습니다!"
		EndingType.COACH_PATH:
			return "리더십과 전술 이해도로 훌륭한 지도자의 길을 걷게 되었습니다!"
		EndingType.HIDDEN_LEGEND:
			return "전설적인 순간들을 만들어낸 진정한 레전드가 되었습니다!"
		_:
			return "3년간의 고교 축구 생활을 마쳤습니다."


# 이벤트 핸들러들
func _on_match_completed(success_or_result, maybe_result = null):
	var match_data: Dictionary = {}
	var success := true

	if maybe_result == null:
		if success_or_result is Dictionary:
			match_data = success_or_result
		else:
			return
	else:
		success = bool(success_or_result)
		match_data = maybe_result

	if not success:
		return
	# MVP 체크
	if match_data.get("mvp_player", "") == "player":
		current_achievements.mvp_count += 1
		ending_condition_met.emit("mvp_count", current_achievements.mvp_count)

	# 골 수 체크
	var goals = match_data.get("player_goals", 0)
	current_achievements.goals_season += goals

	# 완벽한 경기 체크 (모든 스탯 90% 이상)
	if match_data.get("performance_rating", 0) >= 95:
		current_achievements.perfect_matches += 1
		ending_condition_met.emit("perfect_matches", current_achievements.perfect_matches)


func _on_season_completed(season_data: Dictionary):
	# 시즌 골 초기화
	current_achievements.goals_season = 0

	# 주장 임명 체크
	if season_data.get("captain", false):
		current_achievements.team_captain = true
		ending_condition_met.emit("team_captain", true)


func _on_special_event(event_data: Dictionary):
	var event_type = event_data.get("type", "")

	match event_type:
		"international_match":
			current_achievements.international_exp = true
			ending_condition_met.emit("international_exp", true)
		"coaching_practice":
			current_achievements.coaching_exp = true
			ending_condition_met.emit("coaching_exp", true)
		"secret_training":
			current_achievements.secret_events += 1
			ending_condition_met.emit("secret_events", current_achievements.secret_events)
		"legendary_moment":
			current_achievements.legendary_moment = true
			ending_condition_met.emit("legendary_moment", true)
		"mentoring_junior":
			current_achievements.mentoring_count += 1
			ending_condition_met.emit("mentoring_count", current_achievements.mentoring_count)


func _on_academic_grade(grade_data: Dictionary):
	# 학업 성적은 PlayerData에서 직접 관리
	pass


# 세이브/로드 지원
func get_save_data() -> Dictionary:
	return {"current_achievements": current_achievements}


func load_save_data(data: Dictionary):
	if data.has("current_achievements"):
		current_achievements = data.current_achievements


# 치트/디버그 함수들 (개발용)
func debug_force_ending(ending_type: EndingType):
	if OS.is_debug_build():
		trigger_ending(ending_type)


func debug_unlock_all_conditions():
	if OS.is_debug_build():
		current_achievements = {
			"mvp_count": 5,
			"goals_season": 30,
			"international_exp": true,
			"coaching_exp": true,
			"mentoring_count": 5,
			"team_captain": true,
			"secret_events": 10,
			"perfect_matches": 5,
			"legendary_moment": true
		}
		print("모든 엔딩 조건 해제됨 (디버그)")
