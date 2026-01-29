extends Node
# Removed class_name to avoid autoload singleton conflict

## Phase 6.4: Story Boss Manager
## 스토리 보스 격파 시 전설 코치 카드 획득 시스템

signal boss_unlocked(boss_id: String, boss_data: Dictionary)
signal boss_battle_started(boss_id: String)
signal boss_defeated(boss_id: String, reward_coach: Resource)
signal boss_failed(boss_id: String, attempt_count: int)

## Boss Data
const STORY_BOSSES: Array = [
	{
		"id": "julian",
		"name": "줄리앙",
		"stage_required": 30,
		"team_ca": 85,
		"description": "도깨비 슛의 달인. 창의적인 드리블로 수비를 농락한다.",
		"reward_coach":
		{
			"id": "julian_coach",
			"name": "줄리앙 (전설)",
			"rarity": 4,  # LEGENDARY
			"category": 0,  # TECHNICAL
			"training_effect_bonus": 0.40,
			"specialty_ability": "미라지 드리블과 도깨비 슛 전수",
			"specialty_ability_type": "DribblingMaster"
		}
	},
	{
		"id": "caesar",
		"name": "시저",
		"stage_required": 50,
		"team_ca": 95,
		"description": "총알 슛의 마스터. 폭발적인 피지컬과 파워 슈팅.",
		"reward_coach":
		{
			"id": "caesar_coach",
			"name": "시저 (전설)",
			"rarity": 4,  # LEGENDARY
			"category": 1,  # PHYSICAL
			"training_effect_bonus": 0.45,
			"specialty_ability": "폭발적 스피드와 총알 슛 전수",
			"specialty_ability_type": "SpeedDemon"
		}
	},
	{
		"id": "shootdoli",
		"name": "슛돌이",
		"stage_required": 70,
		"team_ca": 105,
		"description": "독수리 슛의 전설. 균형 잡힌 능력과 롱샷의 달인.",
		"reward_coach":
		{
			"id": "shootdoli_coach",
			"name": "슛돌이 (전설)",
			"rarity": 4,  # LEGENDARY
			"category": 2,  # MENTAL
			"training_effect_bonus": 0.50,
			"specialty_ability": "모든 훈련 효율 +15%, 롱샷 특기 전수",
			"specialty_ability_type": "ClutchPlayer"
		}
	},
	{
		"id": "finalBoss",
		"name": "최종 보스",
		"stage_required": 100,
		"team_ca": 120,
		"description": "전설의 선수. 모든 능력이 극한에 달한 최강의 적.",
		"reward_coach":
		{
			"id": "legend_coach",
			"name": "전설의 코치",
			"rarity": 4,  # LEGENDARY
			"category": 3,  # TACTICAL
			"training_effect_bonus": 0.60,
			"specialty_ability": "모든 훈련 대성공 확률 +10%, 궁극 기술 전수",
			"specialty_ability_type": "TacticalGenius"
		}
	}
]

## State
var defeated_bosses: Array = []  # 격파한 보스 ID 목록
var unlocked_bosses: Array = []  # 해금된 보스 ID 목록
var boss_attempt_counts: Dictionary = {}  # 보스별 시도 횟수

## Dependencies
var _coach_card_system: Node = null
var _stage_manager: Node = null


func _ready() -> void:
	_coach_card_system = get_node_or_null("/root/CoachCardSystem")
	_stage_manager = get_node_or_null("/root/StageManager")

	# StageManager 신호 연결
	if _stage_manager and _stage_manager.has_signal("stage_cleared"):
		_stage_manager.stage_cleared.connect(_on_stage_cleared)

	print("[StoryBossManager] Initialized with %d bosses" % STORY_BOSSES.size())


## ========== Boss Unlock System ==========


func check_boss_unlocks(current_stage: int) -> Array:
	"""스테이지에 따른 보스 해금 체크.

	Args:
		current_stage: 현재 도달한 스테이지

	Returns:
		새로 해금된 보스 목록
	"""
	var newly_unlocked: Array = []

	for boss_data in STORY_BOSSES:
		var boss_id: String = boss_data.id

		# 이미 해금되었거나 격파한 보스 스킵
		if boss_id in unlocked_bosses or boss_id in defeated_bosses:
			continue

		# 스테이지 요구사항 체크
		if current_stage >= boss_data.stage_required:
			unlocked_bosses.append(boss_id)
			newly_unlocked.append(boss_data)
			boss_unlocked.emit(boss_id, boss_data)
			print("[StoryBossManager] Boss unlocked: %s (Stage %d)" % [boss_data.name, boss_data.stage_required])

	return newly_unlocked


func _on_stage_cleared(stage: int, _result: Dictionary) -> void:
	"""스테이지 클리어 콜백."""
	check_boss_unlocks(stage)


## ========== Boss Battle ==========


func get_available_bosses() -> Array:
	"""도전 가능한 보스 목록 반환."""
	var available: Array = []

	for boss_id in unlocked_bosses:
		if boss_id not in defeated_bosses:
			var boss_data = get_boss_data(boss_id)
			if not boss_data.is_empty():
				available.append(boss_data)

	return available


func get_boss_data(boss_id: String) -> Dictionary:
	"""보스 데이터 조회."""
	for boss_data in STORY_BOSSES:
		if boss_data.id == boss_id:
			return boss_data.duplicate(true)
	return {}


func is_boss_unlocked(boss_id: String) -> bool:
	"""보스 해금 여부."""
	return boss_id in unlocked_bosses


func is_boss_defeated(boss_id: String) -> bool:
	"""보스 격파 여부."""
	return boss_id in defeated_bosses


func start_boss_battle(boss_id: String) -> Dictionary:
	"""보스 전투 시작.

	Args:
		boss_id: 보스 ID

	Returns:
		전투 정보 Dictionary
	"""
	if boss_id not in unlocked_bosses:
		return {"error": "Boss not unlocked"}

	if boss_id in defeated_bosses:
		return {"error": "Boss already defeated"}

	var boss_data = get_boss_data(boss_id)
	if boss_data.is_empty():
		return {"error": "Boss not found"}

	# 시도 횟수 증가
	if not boss_attempt_counts.has(boss_id):
		boss_attempt_counts[boss_id] = 0
	boss_attempt_counts[boss_id] += 1

	boss_battle_started.emit(boss_id)
	print("[StoryBossManager] Boss battle started: %s (Attempt #%d)" % [boss_data.name, boss_attempt_counts[boss_id]])

	return {
		"success": true,
		"boss_id": boss_id,
		"boss_name": boss_data.name,
		"team_ca": boss_data.team_ca,
		"attempt_count": boss_attempt_counts[boss_id],
		"boss_data": boss_data
	}


func on_boss_battle_completed(boss_id: String, victory: bool, match_result: Dictionary = {}) -> Dictionary:
	"""보스 전투 완료 처리.

	Args:
		boss_id: 보스 ID
		victory: 승리 여부
		match_result: 경기 결과 (선택)

	Returns:
		처리 결과 Dictionary
	"""
	var boss_data = get_boss_data(boss_id)
	if boss_data.is_empty():
		return {"error": "Boss not found"}

	var result: Dictionary = {
		"boss_id": boss_id,
		"boss_name": boss_data.name,
		"victory": victory,
		"attempt_count": boss_attempt_counts.get(boss_id, 1),
		"match_result": match_result
	}

	if victory:
		# 보스 격파 처리
		defeated_bosses.append(boss_id)

		# 전설 코치 보상 지급
		var reward_coach = _grant_legendary_coach(boss_data)
		result["reward_coach"] = reward_coach

		boss_defeated.emit(boss_id, reward_coach)
		print(
			(
				"[StoryBossManager] BOSS DEFEATED! %s - Reward: %s"
				% [boss_data.name, reward_coach.coach_name if reward_coach else "None"]
			)
		)
	else:
		# 패배 처리
		boss_failed.emit(boss_id, boss_attempt_counts.get(boss_id, 1))
		print(
			(
				"[StoryBossManager] Boss battle failed: %s (Attempt #%d)"
				% [boss_data.name, boss_attempt_counts.get(boss_id, 1)]
			)
		)

	return result


func _grant_legendary_coach(boss_data: Dictionary) -> Resource:
	"""전설 코치 카드 보상 지급."""
	if not _coach_card_system:
		push_error("[StoryBossManager] CoachCardSystem not found!")
		return null

	var reward_data: Dictionary = boss_data.get("reward_coach", {})
	if reward_data.is_empty():
		return null

	# CoachCard 생성
	var CoachCardClass = preload("res://scripts/model/CoachCard.gd")
	var coach = CoachCardClass.new()

	coach.coach_name = reward_data.get("name", "Unknown Coach")
	coach.rarity = reward_data.get("rarity", CoachCardClass.Rarity.LEGENDARY)
	coach.category = reward_data.get("category", CoachCardClass.Category.TECHNICAL)
	coach.training_effect_bonus = reward_data.get("training_effect_bonus", 0.40)
	coach.specialty_ability = reward_data.get("specialty_ability", "")
	coach.specialty_ability_type = reward_data.get("specialty_ability_type", "")

	# 인벤토리에 추가
	if _coach_card_system.has_method("add_coach_to_inventory"):
		_coach_card_system.add_coach_to_inventory(coach)
		print("[StoryBossManager] Legendary coach added: %s" % coach.coach_name)

	return coach


## ========== Query Methods ==========


func get_next_boss() -> Dictionary:
	"""다음 도전할 보스 데이터."""
	for boss_data in STORY_BOSSES:
		var boss_id: String = boss_data.id
		if boss_id not in defeated_bosses:
			return boss_data.duplicate(true)
	return {}


func get_boss_progress() -> Dictionary:
	"""보스 진행 상황."""
	return {
		"total_bosses": STORY_BOSSES.size(),
		"unlocked_count": unlocked_bosses.size(),
		"defeated_count": defeated_bosses.size(),
		"unlocked_bosses": unlocked_bosses.duplicate(),
		"defeated_bosses": defeated_bosses.duplicate()
	}


func get_attempt_count(boss_id: String) -> int:
	"""보스 시도 횟수."""
	return boss_attempt_counts.get(boss_id, 0)


## ========== Save/Load ==========


func save_state() -> Dictionary:
	"""상태 저장."""
	return {
		"defeated_bosses": defeated_bosses.duplicate(),
		"unlocked_bosses": unlocked_bosses.duplicate(),
		"boss_attempt_counts": boss_attempt_counts.duplicate()
	}


func load_state(data: Dictionary) -> void:
	"""상태 로드."""
	defeated_bosses = data.get("defeated_bosses", [])
	unlocked_bosses = data.get("unlocked_bosses", [])
	boss_attempt_counts = data.get("boss_attempt_counts", {})
	print(
		"[StoryBossManager] State loaded: %d defeated, %d unlocked" % [defeated_bosses.size(), unlocked_bosses.size()]
	)


## ========== Debug ==========


func debug_unlock_boss(boss_id: String) -> void:
	"""디버그: 보스 강제 해금."""
	if boss_id not in unlocked_bosses:
		unlocked_bosses.append(boss_id)
		var boss_data = get_boss_data(boss_id)
		if not boss_data.is_empty():
			boss_unlocked.emit(boss_id, boss_data)
		print("[StoryBossManager] DEBUG: Boss unlocked: %s" % boss_id)


func debug_defeat_boss(boss_id: String) -> void:
	"""디버그: 보스 강제 격파 (보상 포함)."""
	if boss_id not in unlocked_bosses:
		debug_unlock_boss(boss_id)

	if boss_id not in defeated_bosses:
		var boss_data = get_boss_data(boss_id)
		defeated_bosses.append(boss_id)
		var reward = _grant_legendary_coach(boss_data)
		boss_defeated.emit(boss_id, reward)
		print("[StoryBossManager] DEBUG: Boss defeated: %s" % boss_id)


func debug_print_status() -> void:
	"""디버그: 상태 출력."""
	print("\n" + "=".repeat(50))
	print("[StoryBossManager] Status")
	print("=".repeat(50))
	print("Total bosses: %d" % STORY_BOSSES.size())
	print("Unlocked: %s" % str(unlocked_bosses))
	print("Defeated: %s" % str(defeated_bosses))
	print("Attempts: %s" % str(boss_attempt_counts))
	print("\nBoss list:")
	for boss in STORY_BOSSES:
		var status: String = "LOCKED"
		if boss.id in defeated_bosses:
			status = "DEFEATED"
		elif boss.id in unlocked_bosses:
			status = "AVAILABLE"
		print("  - %s [Stage %d] - %s" % [boss.name, boss.stage_required, status])
	print("=".repeat(50) + "\n")
