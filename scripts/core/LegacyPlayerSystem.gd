extends Node

# Legacy Player System - 레거시 선수 시스템 (Week 10 고급 기능)
# 졸업한 선배들, 전설적인 선수들과의 만남과 멘토링 시스템

signal legacy_player_met(player_data: Dictionary)
signal mentoring_session_started(session_data: Dictionary)
signal special_technique_learned(technique: Dictionary)
signal legacy_story_unlocked(story: Dictionary)

# 레거시 선수 타입
enum LegacyType { ACADEMY_SENIOR, CLUB_LEGEND, NATIONAL_HERO, WORLD_STAR, COACH_PLAYER }  # 아카데미 선배 (졸업생)  # 클럽 레전드  # 국가대표 출신  # 해외 진출 성공자  # 현재 코치로 활동 중인 전 선수

# 레거시 선수 데이터베이스
var legacy_players = {
	"kim_senior":
	{
		"type": LegacyType.ACADEMY_SENIOR,
		"name": "김선배",
		"position": "CM",
		"graduation_year": 2020,
		"achievements": ["전국대회 준우승", "지역리그 MVP"],
		"current_status": "대학교 선수",
		"specialty": "passing_master",
		"personality": "friendly",
		"story": "3년 전 우리 아카데미를 졸업한 중앙 미드필더. 패스 능력으로 유명했다.",
		"techniques": ["curved_pass", "through_pass"],
		"mentoring_available": true,
		"relationship": 0.0,
		"last_met": 0
	},
	"park_legend":
	{
		"type": LegacyType.CLUB_LEGEND,
		"name": "박레전드",
		"position": "ST",
		"graduation_year": 2010,
		"achievements": ["K리그 득점왕", "아시아챔피언스리그 우승"],
		"current_status": "프로선수 은퇴",
		"specialty": "goal_scorer",
		"personality": "inspiring",
		"story": "우리 아카데미 출신으로 K리그에서 큰 성공을 거둔 전설적인 스트라이커.",
		"techniques": ["power_shot", "positioning_sense"],
		"mentoring_available": true,
		"relationship": 0.0,
		"last_met": 0
	},
	"lee_national":
	{
		"type": LegacyType.NATIONAL_HERO,
		"name": "이국대",
		"position": "CB",
		"graduation_year": 2005,
		"achievements": ["월드컵 출전", "아시안컵 우승", "국가대표 50경기"],
		"current_status": "축구해설위원",
		"specialty": "defensive_master",
		"personality": "serious",
		"story": "우리 아카데미가 자랑하는 국가대표 출신 수비수. 월드컵 무대까지 경험했다.",
		"techniques": ["tactical_defending", "leadership_aura"],
		"mentoring_available": true,
		"relationship": 0.0,
		"last_met": 0
	},
	"choi_world":
	{
		"type": LegacyType.WORLD_STAR,
		"name": "최월드",
		"position": "CAM",
		"graduation_year": 2000,
		"achievements": ["유럽리그 진출", "분데스리가 출전", "국가대표 주장"],
		"current_status": "해외 코치",
		"specialty": "creative_genius",
		"personality": "wise",
		"story": "우리 아카데미 최초로 유럽 진출에 성공한 창조적인 미드필더.",
		"techniques": ["no_look_pass", "creative_dribbling"],
		"mentoring_available": false,  # 해외 거주로 만나기 어려움
		"relationship": 0.0,
		"last_met": 0
	}
}

# 현재 활성화된 멘토링 세션
var active_mentoring: Dictionary = {}
var met_players: Array = []
var learned_techniques: Array = []

# 레거시 스토리
var unlocked_stories: Array = []


func _ready():
	print("[LegacyPlayerSystem] Initializing legacy player system")

	# GameManager 신호 연결
	if GameManager:
		GameManager.week_advanced.connect(_on_week_advanced)

	# 초기 데이터 로드
	_load_legacy_data()


func _on_week_advanced(_week: int, _year: int):
	"""주간 진행 시 레거시 이벤트 체크"""
	# 레거시 선수 만남 이벤트 (낮은 확률)
	if randf() < 0.05:  # 5% 확률
		_trigger_random_legacy_encounter()

	# 멘토링 세션 진행
	if not active_mentoring.is_empty():
		_advance_mentoring_session()


func encounter_legacy_player(player_id: String) -> Dictionary:
	"""레거시 선수와의 만남"""
	if not legacy_players.has(player_id):
		return {"success": false, "message": "존재하지 않는 선수입니다."}

	var player = legacy_players[player_id]
	var current_week = GameManager.get_current_week() if GameManager else 1

	# 최근에 만났는지 체크
	if current_week - player.last_met < 4:  # 4주 쿨다운
		return {
			"success": false,
			"message": "%s님은 %d주 후에 다시 만날 수 있습니다." % [player.name, 4 - (current_week - player.last_met)]
		}

	# 만남 이벤트 처리
	player.last_met = current_week

	if not player_id in met_players:
		met_players.append(player_id)

	# 관계도 향상
	player.relationship = min(1.0, player.relationship + 0.1)

	# 만남 결과 생성
	var encounter_result = _generate_encounter_result(player)

	legacy_player_met.emit(player)
	print("[LegacyPlayerSystem] Met legacy player: %s" % player.name)

	return {"success": true, "encounter": encounter_result}


func _generate_encounter_result(player: Dictionary) -> Dictionary:
	"""만남 결과 생성"""
	var result = {"player": player, "interaction_type": "", "rewards": [], "story_unlocked": false}

	# 성격에 따른 상호작용 타입 결정
	match player.personality:
		"friendly":
			result.interaction_type = "casual_chat"
			result.rewards = _generate_casual_rewards(player)
		"inspiring":
			result.interaction_type = "motivational_speech"
			result.rewards = _generate_inspiring_rewards(player)
		"serious":
			result.interaction_type = "intense_training"
			result.rewards = _generate_training_rewards(player)
		"wise":
			result.interaction_type = "wisdom_sharing"
			result.rewards = _generate_wisdom_rewards(player)

	# 스토리 언락 체크
	if player.relationship > 0.5 and not (player.name + "_story") in unlocked_stories:
		result.story_unlocked = true
		unlocked_stories.append(player.name + "_story")
		legacy_story_unlocked.emit({"player": player.name, "story": player.story})

	return result


func _generate_casual_rewards(player: Dictionary) -> Array:
	"""친근한 대화 보상"""
	var rewards = []

	# 기본 동기 증가
	rewards.append({"type": "motivation", "value": 10, "description": "%s님과의 즐거운 대화로 동기가 올랐습니다!" % player.name})

	# 랜덤 팁
	if randf() < 0.3:
		rewards.append(
			{
				"type": "technique_hint",
				"value": _get_random_technique(player),
				"description": "%s님이 유용한 기술 팁을 알려줬습니다." % player.name
			}
		)

	return rewards


func _generate_inspiring_rewards(player: Dictionary) -> Array:
	"""영감을 주는 연설 보상"""
	var rewards = []

	# 큰 동기 증가
	rewards.append({"type": "motivation", "value": 25, "description": "%s님의 격려로 큰 힘을 얻었습니다!" % player.name})

	# 자신감 증가
	if PlayerManager:
		rewards.append({"type": "stat_boost", "stat": "composure", "value": 5, "description": "자신감이 크게 향상되었습니다!"})

	return rewards


func _generate_training_rewards(player: Dictionary) -> Array:
	"""강도 높은 훈련 보상"""
	var rewards = []

	# 전문 분야 훈련 보너스
	var specialty_bonus = _get_specialty_training_bonus(player.specialty)
	rewards.append(
		{
			"type": "training_bonus",
			"category": specialty_bonus.category,
			"value": specialty_bonus.bonus,
			"description": "%s님의 특별 훈련으로 %s 능력이 향상됩니다!" % [player.name, specialty_bonus.category]
		}
	)

	return rewards


func _generate_wisdom_rewards(player: Dictionary) -> Array:
	"""지혜 공유 보상"""
	var rewards = []

	# 전술 이해도 증가
	rewards.append(
		{"type": "tactical_knowledge", "value": 1, "description": "%s님의 경험담으로 축구에 대한 이해가 깊어졌습니다." % player.name}
	)

	# 특수 기술 전수 기회
	if randf() < 0.4:
		var technique = _get_random_technique(player)
		rewards.append(
			{"type": "special_technique", "technique": technique, "description": "%s님이 특별한 기술을 전수해줍니다!" % player.name}
		)

	return rewards


func _get_specialty_training_bonus(specialty: String) -> Dictionary:
	"""전문 분야에 따른 훈련 보너스"""
	match specialty:
		"passing_master":
			return {"category": "technical", "bonus": 0.15}
		"goal_scorer":
			return {"category": "technical", "bonus": 0.2}
		"defensive_master":
			return {"category": "physical", "bonus": 0.15}
		"creative_genius":
			return {"category": "mental", "bonus": 0.2}
		_:
			return {"category": "technical", "bonus": 0.1}


func _get_random_technique(player: Dictionary) -> Dictionary:
	"""플레이어의 기술 중 랜덤 선택"""
	var techniques = player.get("techniques", [])
	if techniques.is_empty():
		return {"name": "basic_technique", "description": "기본 기술"}

	var technique_name = techniques[randi() % techniques.size()]
	return _get_technique_data(technique_name)


func _get_technique_data(technique_name: String) -> Dictionary:
	"""기술 데이터 반환"""
	var technique_database = {
		"curved_pass": {"name": "커브 패스", "description": "공을 휘어서 패스하는 고급 기술", "effect": {"passing": 3, "technique": 2}},
		"through_pass":
		{"name": "스루 패스", "description": "수비수 사이로 정확한 패스를 넣는 기술", "effect": {"passing": 4, "vision": 3}},
		"power_shot": {"name": "파워 슛", "description": "강력한 힘으로 골을 노리는 슛", "effect": {"finishing": 4, "power": 2}},
		"positioning_sense":
		{"name": "포지션 센스", "description": "최적의 위치를 찾는 감각", "effect": {"positioning": 5, "vision": 2}},
		"tactical_defending":
		{"name": "전술적 수비", "description": "지능적인 수비 위치 선정", "effect": {"tackling": 3, "positioning": 4}},
		"leadership_aura": {"name": "리더십 오라", "description": "팀을 이끄는 카리스마", "effect": {"leadership": 5, "teamwork": 3}},
		"no_look_pass": {"name": "노룩 패스", "description": "보지 않고 하는 환상적인 패스", "effect": {"passing": 5, "technique": 4}},
		"creative_dribbling":
		{"name": "창조적 드리블", "description": "예측 불가능한 드리블 기술", "effect": {"dribbling": 5, "creativity": 4}}
	}

	return technique_database.get(
		technique_name, {"name": "기본 기술", "description": "일반적인 축구 기술", "effect": {"technique": 1}}
	)


func start_mentoring_session(player_id: String) -> Dictionary:
	"""멘토링 세션 시작"""
	if not legacy_players.has(player_id):
		return {"success": false, "message": "존재하지 않는 선수입니다."}

	var player = legacy_players[player_id]

	if not player.mentoring_available:
		return {"success": false, "message": "%s님은 현재 멘토링을 할 수 없습니다." % player.name}

	if not active_mentoring.is_empty():
		return {"success": false, "message": "이미 진행 중인 멘토링이 있습니다."}

	if player.relationship < 0.3:
		return {"success": false, "message": "%s님과 더 친해진 후에 멘토링을 받을 수 있습니다." % player.name}

	# 멘토링 세션 설정
	active_mentoring = {
		"player_id": player_id,
		"player": player,
		"start_week": GameManager.get_current_week() if GameManager else 1,
		"duration": randi_range(3, 6),  # 3-6주
		"progress": 0,
		"rewards": _generate_mentoring_rewards(player)
	}

	mentoring_session_started.emit(active_mentoring)
	print("[LegacyPlayerSystem] Started mentoring with: %s" % player.name)

	return {"success": true, "session": active_mentoring}


func _generate_mentoring_rewards(player: Dictionary) -> Dictionary:
	"""멘토링 보상 생성"""
	var rewards = {"weekly_bonus": {}, "final_rewards": []}

	# 주간 보너스 (전문 분야에 따라)
	var bonus_data = _get_specialty_training_bonus(player.specialty)
	rewards.weekly_bonus = {
		"category": bonus_data.category,
		"bonus": bonus_data.bonus / 2,  # 멘토링은 절반 효과
		"description": "%s님의 지도로 %s 훈련 효과가 향상됩니다." % [player.name, bonus_data.category]
	}

	# 완료 시 보상
	rewards.final_rewards = [
		{
			"type": "special_technique",
			"technique": _get_random_technique(player),
			"description": "%s님이 특별한 기술을 완전히 전수해줍니다!" % player.name
		},
		{
			"type": "stat_permanent",
			"stats": _get_mentoring_stat_rewards(player.specialty),
			"description": "%s님의 멘토링으로 영구적인 능력 향상을 얻었습니다!" % player.name
		}
	]

	return rewards


func _get_mentoring_stat_rewards(specialty: String) -> Dictionary:
	"""멘토링 완료 시 능력치 보상"""
	match specialty:
		"passing_master":
			return {"passing": 3, "vision": 2, "technique": 1}
		"goal_scorer":
			return {"finishing": 3, "positioning": 2, "composure": 1}
		"defensive_master":
			return {"tackling": 3, "marking": 2, "positioning": 1}
		"creative_genius":
			return {"dribbling": 2, "creativity": 3, "vision": 2}
		_:
			return {"technique": 2, "positioning": 1}


func _advance_mentoring_session():
	"""멘토링 세션 진행"""
	if active_mentoring.is_empty():
		return

	active_mentoring.progress += 1

	# 주간 보너스 적용
	var weekly_bonus = active_mentoring.rewards.weekly_bonus
	if TrainingManager and weekly_bonus.has("category"):
		TrainingManager.set_category_bonus(weekly_bonus.category, weekly_bonus.bonus)

	print("[LegacyPlayerSystem] Mentoring progress: %d/%d" % [active_mentoring.progress, active_mentoring.duration])

	# 완료 체크
	if active_mentoring.progress >= active_mentoring.duration:
		_complete_mentoring_session()


func _complete_mentoring_session():
	"""멘토링 세션 완료"""
	var final_rewards = active_mentoring.rewards.final_rewards

	for reward in final_rewards:
		_apply_mentoring_reward(reward)

	# 관계도 크게 향상
	var player_id = active_mentoring.player_id
	legacy_players[player_id].relationship = min(1.0, legacy_players[player_id].relationship + 0.2)

	print("[LegacyPlayerSystem] Completed mentoring with: %s" % active_mentoring.player.name)

	# 세션 클리어
	active_mentoring.clear()


func _apply_mentoring_reward(reward: Dictionary):
	"""멘토링 보상 적용"""
	match reward.type:
		"special_technique":
			var technique = reward.technique
			if not technique.name in learned_techniques:
				learned_techniques.append(technique.name)
				special_technique_learned.emit(technique)

				# 능력치 적용
				if PlayerManager and technique.has("effect"):
					for stat in technique.effect:
						PlayerManager.add_experience(stat, technique.effect[stat])

		"stat_permanent":
			if PlayerManager and reward.has("stats"):
				for stat in reward.stats:
					PlayerManager.add_experience(stat, reward.stats[stat])


func _trigger_random_legacy_encounter():
	"""랜덤 레거시 만남 이벤트"""
	var available_players = []

	for player_id in legacy_players:
		var player = legacy_players[player_id]
		var current_week = GameManager.get_current_week() if GameManager else 1

		# 쿨다운이 끝난 선수들만
		if current_week - player.last_met >= 4:
			available_players.append(player_id)

	if available_players.is_empty():
		return

	var selected_player = available_players[randi() % available_players.size()]
	var encounter_result = encounter_legacy_player(selected_player)

	if encounter_result.success:
		# DialogController를 통해 이벤트 표시
		if DialogController:
			var player = legacy_players[selected_player]
			DialogController.start_notification_dialog("레거시 만남!", "%s님을 우연히 만났습니다!" % player.name)


func _load_legacy_data():
	"""레거시 데이터 로드"""
	# 실제로는 세이브 파일에서 로드
	pass


# 공개 API 메서드
func get_legacy_player(player_id: String) -> Dictionary:
	"""레거시 선수 정보 반환"""
	return legacy_players.get(player_id, {}).duplicate()


func get_all_legacy_players() -> Dictionary:
	"""모든 레거시 선수 정보 반환"""
	return legacy_players.duplicate()


func get_met_players() -> Array:
	"""만난 선수 목록 반환"""
	return met_players.duplicate()


func get_learned_techniques() -> Array:
	"""배운 기술 목록 반환"""
	return learned_techniques.duplicate()


func get_unlocked_stories() -> Array:
	"""해금된 스토리 목록 반환"""
	return unlocked_stories.duplicate()


func get_active_mentoring() -> Dictionary:
	"""현재 멘토링 정보 반환"""
	return active_mentoring.duplicate()


func is_mentoring_active() -> bool:
	"""멘토링 활성 여부"""
	return not active_mentoring.is_empty()


# 테스트 함수
func test_legacy_system():
	"""레거시 시스템 테스트"""
	print("=== Legacy Player System Test ===")

	# 레거시 선수와 만남
	for player_id in legacy_players:
		var result = encounter_legacy_player(player_id)
		print("Encounter %s: %s" % [player_id, result])

	# 멘토링 시작
	var mentoring_result = start_mentoring_session("kim_senior")
	print("Mentoring result: %s" % str(mentoring_result))

	# 배운 기술 출력
	print("Learned techniques: %s" % str(get_learned_techniques()))

	print("✅ Legacy system test completed")
