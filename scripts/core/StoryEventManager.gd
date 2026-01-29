extends Node

# Story Event Manager - 메인 스토리 이벤트 관리
# Dialogic과 통합하여 스토리 진행

signal story_event_triggered(event_id: String, choices: Array)
signal story_choice_made(event_id: String, choice_index: int)
signal story_flag_set(flag_name: String, value: bool)
signal emotional_state_changed(state: String, duration: int)

# 스토리 데이터 경로
const CHARACTERS_PATH = "res://data/story_characters.json"
const EVENTS_PATH = "res://data/story_events.json"

# 캐릭터 데이터
var characters: Dictionary = {}
var relationships: Dictionary = {}  # character_id -> relationship_value
var emotional_state: String = "normal"
var emotional_timer: int = 0

# 스토리 플래그
var story_flags: Dictionary = {}
var event_history: Array = []
var current_event: Dictionary = {}

# 고정 이벤트 스케줄
const FIXED_EVENTS = {
	1: ["entrance_ceremony", "first_practice"],  # 1주차
	13: ["first_tournament", "rival_introduction"],  # 13주차
	26: ["midterm_exam", "team_conflict"],  # 26주차
	39: ["summer_camp", "special_training"],  # 39주차
	52: ["championship_qualifier", "year_end"],  # 52주차
	65: ["second_year_start", "new_captain"],  # 65주차
	78: ["prefectural_tournament", "scout_visit"],  # 78주차
	91: ["national_qualifier", "injury_crisis"],  # 91주차
	104: ["winter_championship", "second_year_end"],  # 104주차
	117: ["final_year_start", "captain_decision"],  # 117주차
	130: ["last_summer", "college_scout"],  # 130주차
	143: ["final_tournament", "legacy_moment"],  # 143주차
	156: ["contract", "epilogue"]  # 156주차
}

# 랜덤 이벤트 풀
const RANDOM_EVENTS = {
	"relationship":
	["teammate_bonding", "rival_challenge", "coach_advice", "captain_talk", "friend_support", "manager_help"],
	"training":
	[
		"breakthrough_moment",
		"technique_discovery",
		"mental_training",
		"position_change",
		"special_coach",
		"alumni_visit"
	],
	"academy":
	["class_event", "cultural_festival", "study_group", "student_council", "club_recruitment", "exam_stress"],
	"personal": ["family_visit", "injury_scare", "confidence_crisis", "love_interest", "future_worry", "hometown_news"]
}


func _ready():
	load_story_data()
	initialize_relationships()
	print("Story Event Manager initialized")


func load_story_data():
	"""스토리 데이터 로드"""
	# 캐릭터 데이터 로드
	var file = FileAccess.open(CHARACTERS_PATH, FileAccess.READ)
	if file:
		var json_text = file.get_as_text()
		file.close()
		var json = JSON.new()
		var parse_result = json.parse(json_text)
		if parse_result == OK:
			var data = json.data
			characters = data.get("main_characters", {})
			print("Loaded %d characters" % characters.size())


func initialize_relationships():
	"""관계도 초기화"""
	for char_id in characters:
		relationships[char_id] = 0  # 모든 관계를 0에서 시작


func trigger_story_event(week: int):
	"""주차별 스토리 이벤트 트리거"""
	var events_to_trigger = []

	# 고정 이벤트 체크
	if FIXED_EVENTS.has(week):
		events_to_trigger.append_array(FIXED_EVENTS[week])

	# 랜덤 이벤트 체크 (20% 확률)
	if randf() < 0.2:
		var category = ["relationship", "training", "academy", "personal"].pick_random()
		var event = RANDOM_EVENTS[category].pick_random()
		if not event in event_history:  # 중복 방지
			events_to_trigger.append(event)

	# 관계 기반 이벤트 체크
	for char_id in relationships:
		if relationships[char_id] >= 60 and randf() < 0.1:
			var character = characters[char_id]
			if character.has("relationship_events"):
				for event in character.relationship_events:
					if check_event_conditions(event):
						events_to_trigger.append(event)
						break

	# 이벤트 실행
	for event_id in events_to_trigger:
		execute_story_event(event_id)


func execute_story_event(event_id: String):
	"""스토리 이벤트 실행"""
	current_event = create_event_data(event_id)
	event_history.append(event_id)

	# Dialogic 타임라인 시작 (있는 경우)
	if has_dialogic_timeline(event_id):
		start_dialogic_timeline(event_id)
	else:
		# 기본 이벤트 처리
		show_event_dialog(current_event)

	story_event_triggered.emit(event_id, current_event.get("choices", []))


func create_event_data(event_id: String) -> Dictionary:
	"""이벤트 데이터 생성"""
	var event = {
		"id": event_id,
		"title": get_event_title(event_id),
		"description": get_event_description(event_id),
		"choices": get_event_choices(event_id),
		"effects": get_event_effects(event_id),
		"characters": get_event_characters(event_id),
		"background": get_event_background(event_id),
		"bgm": get_event_bgm(event_id)
	}
	return event


func get_event_title(event_id: String) -> String:
	"""이벤트 제목 반환"""
	var titles = {
		"entrance_ceremony": "입학식",
		"first_practice": "첫 연습",
		"rival_introduction": "라이벌 등장",
		"first_tournament": "첫 대회",
		"team_conflict": "팀 내 갈등",
		"summer_camp": "여름 합숙",
		"championship_qualifier": "챔피언십 예선",
		"contract": "계약식"
	}
	return titles.get(event_id, event_id.capitalize())


func get_event_description(event_id: String) -> String:
	"""이벤트 설명 반환"""
	var descriptions = {
		"entrance_ceremony": "고등학교 축구부에 입부하게 된 첫날, 새로운 팀원들과 만나게 됩니다.",
		"first_practice": "첫 훈련에서 자신의 실력을 보여줄 기회가 왔습니다.",
		"rival_introduction": "강력한 라이벌 이민호가 등장합니다. 그는 당신을 도발합니다.",
		"contract": "3년간의 여정이 끝났습니다. 이제 새로운 시작을 준비할 때입니다."
	}
	return descriptions.get(event_id, "")


func get_event_choices(event_id: String) -> Array:
	"""이벤트 선택지 반환"""
	var choices_map = {
		"entrance_ceremony":
		[
			{"text": "자신있게 인사한다", "effect": "confidence"},
			{"text": "조용히 지켜본다", "effect": "observant"},
			{"text": "친구를 찾는다", "effect": "social"}
		],
		"rival_introduction":
		[
			{"text": "도전을 받아들인다", "effect": "competitive"},
			{"text": "무시한다", "effect": "focused"},
			{"text": "친구가 되자고 제안한다", "effect": "diplomatic"}
		]
	}
	return choices_map.get(event_id, [])


func get_event_effects(event_id: String) -> Dictionary:
	"""이벤트 효과 반환"""
	var effects_map = {
		"entrance_ceremony":
		{
			"confidence": {"leadership": 5, "determination": 3},
			"observant": {"vision": 5, "decisions": 3},
			"social": {"teamwork": 5, "relationship_all": 5}
		},
		"rival_introduction":
		{
			"competitive": {"determination": 10, "aggression": 5, "relationship_lee_minho": -10},
			"focused": {"concentration": 10, "composure": 5},
			"diplomatic": {"teamwork": 5, "relationship_lee_minho": 10}
		}
	}
	return effects_map.get(event_id, {})


func get_event_characters(event_id: String) -> Array:
	"""이벤트 등장 캐릭터 반환"""
	var characters_map = {
		"entrance_ceremony": ["coach_park", "jung_sooyeon", "kim_jimin"],
		"rival_introduction": ["lee_minho"],
		"team_conflict": ["jung_sooyeon", "alex_kim", "kang_minsoo"],
		"contract": ["coach_park", "jung_sooyeon", "na_youngho"]
	}
	return characters_map.get(event_id, [])


func get_event_background(event_id: String) -> String:
	"""이벤트 배경 이미지 경로 반환"""
	var backgrounds = {
		"entrance_ceremony": "res://assets/backgrounds/academy_gate.png",
		"first_practice": "res://assets/backgrounds/training_field.png",
		"summer_camp": "res://assets/backgrounds/camp_field.png",
		"contract": "res://assets/backgrounds/ceremony_hall.png"
	}
	return backgrounds.get(event_id, "res://assets/backgrounds/default.png")


func get_event_bgm(event_id: String) -> String:
	"""이벤트 BGM 경로 반환"""
	var bgm_map = {
		"entrance_ceremony": "res://assets/audio/bgm/hopeful.ogg",
		"rival_introduction": "res://assets/audio/bgm/tension.ogg",
		"championship_qualifier": "res://assets/audio/bgm/epic.ogg",
		"contract": "res://assets/audio/bgm/emotional.ogg"
	}
	return bgm_map.get(event_id, "res://assets/audio/bgm/daily.ogg")


func has_dialogic_timeline(event_id: String) -> bool:
	"""Dialogic 타임라인 존재 여부 확인"""
	var timeline_path = "res://dialogic/timelines/%s.dtl" % event_id
	return FileAccess.file_exists(timeline_path)


func start_dialogic_timeline(event_id: String):
	"""Dialogic 타임라인 시작"""
	# Dialogic이 설치되어 있다면
	if get_node_or_null("/root/Dialogic"):
		var Dialogic = get_node("/root/Dialogic")
		Dialogic.start(event_id)

		# 변수 설정
		for char_id in relationships:
			Dialogic.set_variable("relationship_" + char_id, relationships[char_id])

		# 플래그 설정
		for flag in story_flags:
			Dialogic.set_variable(flag, story_flags[flag])


func show_event_dialog(event: Dictionary):
	"""기본 이벤트 다이얼로그 표시"""
	# UI Manager를 통해 다이얼로그 표시
	if UIManager:
		UIManager.show_story_dialog(event)


func make_choice(event_id: String, choice_index: int):
	"""선택지 선택 처리"""
	if not current_event.is_empty():
		var choices = current_event.get("choices", [])
		if choice_index < choices.size():
			var choice = choices[choice_index]
			var effect_key = choice.get("effect", "")

			# 효과 적용
			apply_event_effects(current_event.id, effect_key)

			story_choice_made.emit(event_id, choice_index)


func apply_event_effects(event_id: String, effect_key: String):
	"""이벤트 효과 적용"""
	var effects = get_event_effects(event_id)
	if effects.has(effect_key):
		var effect_data = effects[effect_key]

		# 스킬 포인트 적용
		for skill in effect_data:
			if skill == "relationship_all":
				# 모든 관계도 상승
				for char_id in relationships:
					update_relationship(char_id, effect_data[skill])
			elif skill.begins_with("relationship_"):
				# 특정 캐릭터 관계도
				var char_id = skill.replace("relationship_", "")
				update_relationship(char_id, effect_data[skill])
			else:
				# 일반 스킬
				# TODO: EnhancedPlayerData 정의 필요
				# if EnhancedPlayerData:
				#	EnhancedPlayerData.add_skill_delta(skill, effect_data[skill])
				pass


func update_relationship(character_id: String, delta: int):
	"""캐릭터 관계도 업데이트"""
	if relationships.has(character_id):
		relationships[character_id] = clamp(relationships[character_id] + delta, -100, 100)

		# 관계 레벨 체크
		check_relationship_milestone(character_id)


func check_relationship_milestone(character_id: String):
	"""관계도 마일스톤 체크"""
	var level = relationships[character_id]

	# 관계 레벨별 이벤트 트리거
	if level == 20 and not has_story_flag("relation_20_" + character_id):
		set_story_flag("relation_20_" + character_id, true)
		trigger_relationship_event(character_id, "acquaintance")
	elif level == 60 and not has_story_flag("relation_60_" + character_id):
		set_story_flag("relation_60_" + character_id, true)
		trigger_relationship_event(character_id, "friend")
	elif level == 100 and not has_story_flag("relation_100_" + character_id):
		set_story_flag("relation_100_" + character_id, true)
		trigger_relationship_event(character_id, "best_friend")


func trigger_relationship_event(character_id: String, level: String):
	"""관계 레벨 이벤트 트리거"""
	var event_id = "%s_%s_event" % [character_id, level]
	execute_story_event(event_id)


func check_event_conditions(event_id: String) -> bool:
	"""이벤트 발생 조건 체크"""
	# 이벤트별 조건 체크
	var conditions = {
		"rival_challenge": func(): return relationships.get("lee_minho", 0) < 0,
		# TODO: EnhancedPlayerData 정의 필요
		"coach_advice": func(): return true,  # EnhancedPlayerData.player_week > 20,
		"scout_visit": func(): return true,  # EnhancedPlayerData.get_ca() > 130,
		"championship_qualifier": func(): return true  # EnhancedPlayerData.player_week == 52
	}

	if conditions.has(event_id):
		return conditions[event_id].call()

	return true


func set_story_flag(flag_name: String, value: bool):
	"""스토리 플래그 설정"""
	story_flags[flag_name] = value
	story_flag_set.emit(flag_name, value)


func has_story_flag(flag_name: String) -> bool:
	"""스토리 플래그 확인"""
	return story_flags.get(flag_name, false)


func set_emotional_state(state: String, duration: int = 3):
	"""감정 상태 설정"""
	emotional_state = state
	emotional_timer = duration
	emotional_state_changed.emit(state, duration)


func update_emotional_timer():
	"""감정 타이머 업데이트 (주마다 호출)"""
	if emotional_timer > 0:
		emotional_timer -= 1
		if emotional_timer == 0:
			emotional_state = "normal"
			emotional_state_changed.emit("normal", 0)


func get_emotional_modifier() -> float:
	"""현재 감정 상태의 훈련 효과 수정자 반환"""
	var modifiers = {
		"happy": 1.1,
		"motivated": 1.15,
		"stressed": 0.9,
		"angry": 0.85,
		"sad": 0.8,
		"confident": 1.2,
		"anxious": 0.95,
		"determined": 1.25,
		"normal": 1.0
	}
	return modifiers.get(emotional_state, 1.0)


func get_relationship_summary() -> Dictionary:
	"""관계도 요약 반환"""
	var summary = {}
	for char_id in relationships:
		var level = relationships[char_id]
		var status = "stranger"

		if level >= 80:
			status = "best_friend"
		elif level >= 60:
			status = "friend"
		elif level >= 40:
			status = "teammate"
		elif level >= 20:
			status = "acquaintance"
		elif level < -20:
			status = "rival"

		summary[char_id] = {"level": level, "status": status, "character": characters.get(char_id, {})}

	return summary


func save_story_data() -> Dictionary:
	"""스토리 데이터 저장"""
	return {
		"relationships": relationships,
		"story_flags": story_flags,
		"event_history": event_history,
		"emotional_state": emotional_state,
		"emotional_timer": emotional_timer
	}


func load_story_data_from_save(data: Dictionary):
	"""저장된 스토리 데이터 로드"""
	relationships = data.get("relationships", {})
	story_flags = data.get("story_flags", {})
	event_history = data.get("event_history", [])
	emotional_state = data.get("emotional_state", "normal")
	emotional_timer = data.get("emotional_timer", 0)


# 디버그 함수
func debug_trigger_event(event_id: String):
	"""디버그: 특정 이벤트 강제 트리거"""
	print("[DEBUG] Forcing event: %s" % event_id)
	execute_story_event(event_id)


func debug_set_relationship(character_id: String, value: int):
	"""디버그: 관계도 강제 설정"""
	print("[DEBUG] Setting relationship %s to %d" % [character_id, value])
	relationships[character_id] = clamp(value, -100, 100)


func debug_print_status():
	"""디버그: 현재 상태 출력"""
	print("=== Story Status ===")
	print("Relationships:", relationships)
	print("Flags:", story_flags)
	print("Emotional State:", emotional_state)
	print("Event History:", event_history.slice(-5))  # 최근 5개
