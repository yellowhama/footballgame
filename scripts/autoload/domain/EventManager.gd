extends Node

## EventManager.gd
## 파워프로 스타일 이벤트 시스템 관리
## 작성일: 2025-10-24
## 버전: 1.0

# ============================================
# 시그널
# ============================================

## 이벤트 시작 시 발생
signal event_started(event_data: Dictionary)

## 이벤트 완료 시 발생
signal event_completed(event_id: String, result: Dictionary)

## 이벤트 효과 적용 시 발생
signal event_effects_applied(effects: Dictionary)

## 캐릭터 호감도 변경 시 발생
signal affection_changed(character_id: String, old_value: int, new_value: int)

## 이벤트 플래그 설정 시 발생
signal flag_set(flag_id: String, value: Variant)

# ============================================
# 상태 변수
# ============================================

## 이벤트 플래그 저장소 (조건 체크용)
var event_flags: Dictionary = {}

## 캐릭터 호감도 (character_id: String -> affection: int 0-100)
var character_affection: Dictionary = {
	"rival_taeyoung": 50, "coach_cheolsu": 50, "friend_minjun": 50, "captain_seojun": 50, "gk_jihun": 50  # 강태양 (라이벌)  # 김철수 (코치)  # 박민준 (친구)  # 이서준 (주장)  # 최지훈 (GK)
}

## 발생한 이벤트 기록 (중복 방지)
var triggered_events: Array[String] = []

## 현재 활성 이벤트
var active_event: Dictionary = {}

## 이벤트 데이터 캐시
var event_cache: Dictionary = {}

# ============================================
# 초기화
# ============================================


func _ready() -> void:
	print("[EventManager] Initialized")
	_load_event_data()

	# DateManager 시그널 연결
	_connect_to_date_manager()


func _load_event_data() -> void:
	"""이벤트 데이터 로드 (JSON)"""
	event_cache = {}

	# Load character arc JSON files
	var arc_files = [
		"res://data/events/rival_kang_taeyang_arc.json",
		"res://data/events/friendship_park_minjun_arc.json",
		"res://data/events/mentor_kim_cheolsu_arc.json",
		"res://data/events/captain_lee_seojun_arc.json",
		"res://data/events/guardian_choi_jihun_arc.json"
	]

	var total_events = 0

	for file_path in arc_files:
		var events_loaded = _load_arc_file(file_path)
		total_events += events_loaded

	print("[EventManager] Event data loaded: %d events from %d arc files" % [total_events, arc_files.size()])


func _connect_to_date_manager() -> void:
	"""DateManager 시그널에 연결"""
	if not DateManager:
		print("[EventManager] WARNING: DateManager not found")
		return

	# turn_decision_required: 턴 시작 시 pre-event 체크
	if not DateManager.turn_decision_required.is_connected(_on_turn_decision_required):
		DateManager.turn_decision_required.connect(_on_turn_decision_required)

	# turn_completed: 턴 완료 시 post-event 체크
	if not DateManager.turn_completed.is_connected(_on_turn_completed):
		DateManager.turn_completed.connect(_on_turn_completed)

	print("[EventManager] Connected to DateManager signals")


# ============================================
# 이벤트 트리거 & 처리
# ============================================


func get_next_eligible_event(_timing: String, _week: int) -> Dictionary:
	"""
	현재 조건에 맞는 다음 이벤트를 찾아 반환

	@param _timing: "pre" (훈련/경기 전) 또는 "post" (훈련/경기 후) (TODO: implement)
	@param _week: 현재 주차 (1-156) (TODO: implement)
	@return: 이벤트 데이터 Dictionary (없으면 빈 Dictionary)
	"""

	# TODO: Week 2에서 실제 이벤트 데이터 체크 로직 구현
	# 현재는 빈 Dictionary 반환

	# 우선순위:
	# 1. 고정 이벤트 (fixed_events) - 특정 주차에 반드시 발생
	# 2. 캐릭터 이벤트 (character_events) - 호감도/플래그 조건
	# 3. 콤보 이벤트 (combo_events) - 여러 캐릭터 조건
	# 4. 랜덤 이벤트 (random_events) - 확률 기반

	return {}


func check_event_conditions(event_data: Dictionary) -> bool:
	"""
	이벤트 발생 조건 체크

	@param event_data: 이벤트 데이터
	@return: 조건 충족 여부
	"""

	# 이미 발생한 이벤트는 제외 (once: true인 경우)
	if event_data.get("once", true):
		if event_data["event_id"] in triggered_events:
			return false

	# 조건 체크
	var conditions = event_data.get("conditions", {})

	# 주차 조건
	if conditions.has("week_min"):
		if GameManager.current_week < conditions["week_min"]:
			return false

	if conditions.has("week_max"):
		if GameManager.current_week > conditions["week_max"]:
			return false

	# 호감도 조건
	if conditions.has("affection"):
		for character_id in conditions["affection"]:
			var required = conditions["affection"][character_id]
			var current = character_affection.get(character_id, 50)
			if current < required:
				return false

	# 플래그 조건
	if conditions.has("flags"):
		for flag_id in conditions["flags"]:
			if not event_flags.get(flag_id, false):
				return false

	# CA 조건
	if conditions.has("ca_min"):
		if PlayerData.ca < conditions["ca_min"]:
			return false

	# Position Level 조건
	if conditions.has("position_level_min"):
		# TODO: PositionManager 연동 (Week 1 Day 5)
		pass

	return true


func trigger_event(event_data: Dictionary) -> Dictionary:
	"""
	이벤트 발동 및 기본 선택 처리

	@param event_data: 이벤트 데이터 (event_id/title/choices/effects 등)
	@return: 플레이어 선택 결과 Dictionary
	"""
	var event_id: String = str(event_data.get("event_id", event_data.get("id", "")))
	var title: String = event_data.get("title", "")

	print("[EventManager] Event triggered: %s (%s)" % [event_id, title])

	# 활성 이벤트 업데이트
	active_event = event_data

	# 즉시 적용 효과 처리 (top-level effects)
	var base_effects: Dictionary = event_data.get("effects", {})
	if not base_effects.is_empty():
		_apply_event_effects(base_effects)

	# 시그널 발행
	event_started.emit(event_data)

	# 기본 선택(첫 번째 선택지) 처리
	var choices: Array = event_data.get("choices", [])
	var choice_index := 0
	var choice_data: Dictionary = choices[choice_index] if choices.size() > 0 else {}

	var result: Dictionary = {
		"event_id": event_id, "choice_index": choice_index, "choice_data": choice_data, "success": true
	}

	if not choice_data.is_empty():
		apply_event_effects(event_data, result)

	# 재발동 제한 플래그 처리
	if event_data.get("once", true) and event_id != "" and event_id not in triggered_events:
		triggered_events.append(event_id)

	event_completed.emit(event_id, result)
	active_event = {}

	return result


func apply_event_effects(_event_data: Dictionary, choice_result: Dictionary) -> void:
	"""
	이벤트 효과 적용 (능력치, 호감도, 플래그 등)

	@param _event_data: 이벤트 데이터 (reserved for future use)
	@param choice_result: 플레이어 선택 결과
	"""

	var effects = choice_result.get("choice_data", {}).get("effects", {})

	if effects.is_empty():
		return

	print("[EventManager] Applying effects: ", effects)

	# 능력치 변화
	if effects.has("stat_changes"):
		_apply_stat_changes(effects["stat_changes"])

	# 호감도 변화
	if effects.has("affection_changes"):
		_apply_affection_changes(effects["affection_changes"])

	# 플래그 설정
	if effects.has("flags"):
		var flags_data = effects["flags"]
		if flags_data is Dictionary:
			_apply_flags(flags_data)
		elif flags_data is Array:
			# Convert array to dictionary format
			var flags_dict = {}
			for flag_item in flags_data:
				if flag_item is Dictionary and flag_item.has("id"):
					flags_dict[flag_item["id"]] = flag_item.get("value", true)
			if flags_dict.size() > 0:
				_apply_flags(flags_dict)

	# CA/PA 변화
	if effects.has("ca_change"):
		PlayerData.ca = clamp(PlayerData.ca + effects["ca_change"], 0, 200)

	if effects.has("pa_change"):
		PlayerData.pa = clamp(PlayerData.pa + effects["pa_change"], 0, 200)

	# 컨디션 변화
	if effects.has("condition_change"):
		PlayerData.condition = clamp(PlayerData.condition + effects["condition_change"], 0, 100)

	# 피로도 변화
	if effects.has("fatigue_change"):
		PlayerData.fatigue = clamp(PlayerData.fatigue + effects["fatigue_change"], 0, 100)

	event_effects_applied.emit(effects)


func _apply_stat_changes(stat_changes: Dictionary) -> void:
	"""능력치 변화 적용"""
	for stat_name in stat_changes:
		var change = stat_changes[stat_name]

		# Technical 능력치
		if PlayerData.technical_stats.has(stat_name):
			var old_value = PlayerData.technical_stats[stat_name]
			PlayerData.technical_stats[stat_name] = clamp(old_value + change, 1, 20)

		# Mental 능력치
		elif PlayerData.mental_stats.has(stat_name):
			var old_value = PlayerData.mental_stats[stat_name]
			PlayerData.mental_stats[stat_name] = clamp(old_value + change, 1, 20)

		# Physical 능력치
		elif PlayerData.physical_stats.has(stat_name):
			var old_value = PlayerData.physical_stats[stat_name]
			PlayerData.physical_stats[stat_name] = clamp(old_value + change, 1, 20)


func _apply_affection_changes(affection_changes: Dictionary) -> void:
	"""호감도 변화 적용"""
	for character_id in affection_changes:
		var change = affection_changes[character_id]
		var old_value = character_affection.get(character_id, 50)
		var new_value = clamp(old_value + change, 0, 100)

		character_affection[character_id] = new_value
		affection_changed.emit(character_id, old_value, new_value)

		print("[EventManager] Affection changed: ", character_id, " ", old_value, " -> ", new_value)


func _apply_flags(flags: Dictionary) -> void:
	"""이벤트 플래그 설정"""
	for flag_id in flags:
		var value = flags[flag_id]
		event_flags[flag_id] = value
		flag_set.emit(flag_id, value)

		print("[EventManager] Flag set: ", flag_id, " = ", value)


# ============================================
# 타이밍별 이벤트 처리
# ============================================


func process_pre_events() -> void:
	"""
	훈련/경기 전 이벤트 처리
	DateManager의 turn_decision_required 시그널에서 호출
	"""

	var event = get_next_eligible_event("pre", GameManager.current_week)

	if not event.is_empty():
		await trigger_event(event)


func process_post_events() -> void:
	"""
	훈련/경기 후 이벤트 처리
	DateManager의 turn_completed 시그널에서 호출
	"""

	var event = get_next_eligible_event("post", GameManager.current_week)

	if not event.is_empty():
		await trigger_event(event)


# ============================================
# DateManager 시그널 핸들러
# ============================================


func _on_turn_decision_required(turn_type: String, _turn_info: Dictionary) -> void:
	"""
	DateManager turn_decision_required 시그널 핸들러
	턴 시작 시 pre-event 체크
	"""

	# training 또는 match 턴에서만 pre-event 실행
	if turn_type in ["training", "match"]:
		print("[EventManager] Checking pre-events for turn: ", turn_type)
		await process_pre_events()


func _on_turn_completed(turn_results: Dictionary) -> void:
	"""
	DateManager turn_completed 시그널 핸들러
	턴 완료 시 post-event 체크
	"""

	var turn_type = turn_results.get("turn_type", "")

	# training 또는 match 턴에서만 post-event 실행
	if turn_type in ["training", "match"]:
		print("[EventManager] Checking post-events for turn: ", turn_type)
		await process_post_events()

	# 보상 카드 자동 체크 (Phase 1 Task 1.3)
	check_reward_cards()


# ============================================
# 유틸리티
# ============================================


func get_affection(character_id: String) -> int:
	"""캐릭터 호감도 조회"""
	return character_affection.get(character_id, 50)


func set_affection(character_id: String, value: int) -> void:
	"""캐릭터 호감도 설정"""
	var old_value = character_affection.get(character_id, 50)
	character_affection[character_id] = clamp(value, 0, 100)
	affection_changed.emit(character_id, old_value, character_affection[character_id])


func modify_affection(character_id: String, delta: int) -> void:
	"""호감도 증감"""
	var current = get_affection(character_id)
	set_affection(character_id, current + delta)


func get_flag(flag_id: String, default_value: Variant = false) -> Variant:
	"""이벤트 플래그 조회"""
	return event_flags.get(flag_id, default_value)


func set_flag(flag_id: String, value: Variant) -> void:
	"""이벤트 플래그 설정"""
	event_flags[flag_id] = value
	flag_set.emit(flag_id, value)


func has_triggered_event(event_id: String) -> bool:
	"""이벤트 발생 여부 확인"""
	return event_id in triggered_events


func reset_all_events() -> void:
	"""모든 이벤트 상태 초기화 (새 게임 시작용)"""
	event_flags.clear()
	triggered_events.clear()

	# 호감도 초기화
	for character_id in character_affection:
		character_affection[character_id] = 50

	print("[EventManager] All events reset")


# ============================================
# 디버그 & 테스트
# ============================================


func _debug_print_status() -> void:
	"""현재 상태 출력 (디버그용)"""
	print("=== EventManager Status ===")
	print("Triggered Events: ", triggered_events.size())
	print("Active Flags: ", event_flags.size())
	print("Affection:")
	for character_id in character_affection:
		print("  ", character_id, ": ", character_affection[character_id])
	print("===========================")


# ============================================
# Reward Card Checks (Phase 1 Task 1.3)
# ============================================


func _check_tutorial_completion() -> void:
	"""
	Week 5: 훈련 튜토리얼 완료 시 보상 카드 지급
	"""
	if GameManager.current_week != 5:
		return

	if get_flag("tutorial_card_acquired"):
		return

	# 튜토리얼 완료 조건: 훈련 1회 이상 실행 (간단한 조건으로 변경)
	# GameManager에 total_trainings_completed가 없을 수 있으므로 간단히 Week 5 도달만으로 지급
	var card = DeckManager.load_card_from_json("tutorial_physical_001")
	if card:
		DeckManager.add_card_to_deck(card)
		set_flag("tutorial_card_acquired", true)
		print("[EventManager] Tutorial reward card acquired!")
		# TODO: UI 알림 표시


func _check_debut_match() -> void:
	"""
	Week 15: 첫 경기 출전 시 보상 카드 지급
	"""
	if GameManager.current_week != 15:
		return

	if get_flag("debut_card_acquired"):
		return

	# 데뷔전 조건: Week 15 도달 (경기 출전 기록은 별도 시스템 필요)
	var card = DeckManager.load_card_from_json("debut_mental_001")
	if card:
		DeckManager.add_card_to_deck(card)
		set_flag("debut_card_acquired", true)
		print("[EventManager] Debut match reward card acquired!")
		# TODO: UI 알림 표시


func _check_coach_approval() -> void:
	"""
	Week 30: 김철수 호감도 40+ 시 기본 카드 지급
	"""
	if GameManager.current_week != 30:
		return

	if get_flag("coach_basic_card_acquired"):
		return

	var kim_affection = get_affection("coach_kim")
	if kim_affection >= 40:
		var card = DeckManager.load_card_from_json("coach_kim_basic")
		if card:
			DeckManager.add_card_to_deck(card)
			set_flag("coach_basic_card_acquired", true)
			print("[EventManager] Coach Kim basic card acquired! (Affection: %d)" % kim_affection)
			# TODO: UI 알림 표시


func _check_growth_milestone() -> void:
	"""
	Week 50: CA 100 달성 시 보상 카드 지급
	"""
	if GameManager.current_week != 50:
		return

	if get_flag("growth_milestone_card_acquired"):
		return

	if PlayerData.ca >= 100:
		var card = DeckManager.load_card_from_json("growth_milestone")
		if card:
			DeckManager.add_card_to_deck(card)
			set_flag("growth_milestone_card_acquired", true)
			print("[EventManager] Growth milestone card acquired! (CA: %d)" % PlayerData.ca)
			# TODO: UI 알림 표시


func check_reward_cards() -> void:
	"""
	모든 보상 카드 조건 체크 (Week 시작 시 호출)
	"""
	_check_tutorial_completion()
	_check_debut_match()
	_check_coach_approval()
	_check_growth_milestone()


# ========== Phase 2: Position-Based Route System ==========


func get_available_routes() -> Array[String]:
	"""
	포지션에 따라 사용 가능한 루트 반환
	@return: 루트 ID 배열 (예: ["rival", "friendship", "mentor", "captain"])
	"""
	if not PositionManager:
		push_warning("[EventManager] PositionManager not found, returning all routes")
		return ["rival", "friendship", "mentor", "captain", "guardian"]

	var position = PositionManager.get_primary_position()
	var routes: Array[String] = []

	match position:
		"GK":
			# GK: 라이벌 루트는 GK 변형 버전
			routes = ["rival_gk", "friendship", "mentor", "guardian"]

		"DF", "DM":
			# DF/DM: 라이벌 루트는 DF 변형 버전
			routes = ["rival_def", "friendship", "mentor", "captain", "guardian"]

		"FW", "AM", "WM", "MF":
			# FW/AM/WM: 기본 라이벌 루트 (강태양)
			routes = ["rival", "friendship", "mentor", "captain"]

		_:
			# 기타 포지션: 우정, 멘토만 가능
			routes = ["friendship", "mentor"]

	print("[EventManager] Available routes for position '%s': %s" % [position, routes])
	return routes


func get_rival_character() -> String:
	"""
	포지션에 따른 라이벌 캐릭터 반환
	@return: 라이벌 캐릭터 ID
	"""
	if not PositionManager:
		return "kang_taeyang"  # 기본값

	var position = PositionManager.get_primary_position()

	match position:
		"GK":
			return "park_seongho"  # 박성호 (GK 라이벌)
		"DF", "DM":
			return "lee_minho"  # 이민호 (DF 라이벌)
		_:
			return "kang_taeyang"  # 강태양 (FW 라이벌, 기본값)


func is_route_available(route_id: String) -> bool:
	"""
	특정 루트가 현재 포지션에서 사용 가능한지 확인
	@param route_id: 루트 ID
	@return: 사용 가능 여부
	"""
	var available_routes = get_available_routes()
	return route_id in available_routes


# ========== Phase 2: Branch Point Reward System ==========


func process_branch_reward(route: String, branch_id: int, choice: String) -> void:
	"""
	분기점 선택에 따른 보상 처리 (상호 배타적)
	@param route: 루트 ID ("rival", "friendship", 등)
	@param branch_id: 분기점 번호 (1, 2, 3)
	@param choice: 선택지 ("A", "B", "C")
	"""
	var key = "%s_branch%d_%s" % [route, branch_id, choice]
	print("[EventManager] Processing branch reward: %s" % key)

	match key:
		# ========== 라이벌 루트 ==========
		"rival_branch2_A":
			# A. "정정당당하게 경쟁하자"
			var card = DeckManager.load_card_from_json("kang_taeyang_ssr_rival")
			if card:
				DeckManager.add_card_to_deck(card)
			PlayerData.set_exclusive_trait("rival_awakening", 1)
			set_flag("rival_true_ending_unlocked", true)
			set_flag("rival_branch2_choice", "A")
			print("[EventManager] Rival Route Branch 2 - A: SSR card + Rival Awakening + True Ending")

		"rival_branch2_B":
			# B. "같이 주전 될 방법을 찾자"
			var card = DeckManager.load_card_from_json("dual_ace_sr_tactical")
			if card:
				DeckManager.add_card_to_deck(card)
			# Teamwork 스탯 증가
			if PlayerData:
				var current_teamwork = PlayerData.get_stat("mental", "teamwork")
				PlayerData.set_stat("mental", "teamwork", current_teamwork + 15)
			set_flag("rival_alt_ending_unlocked", true)
			set_flag("rival_branch2_choice", "B")
			print("[EventManager] Rival Route Branch 2 - B: SR card + Teamwork +15 + Alt Ending")

		"rival_branch3_A":
			# A. "내가 에이스다"
			var card = DeckManager.load_card_from_json("absolute_ace_ssr")
			if card:
				DeckManager.add_card_to_deck(card)
			PlayerData.upgrade_exclusive_trait()  # 라이벌 각성 Lv.2 → Lv.3
			var card2 = DeckManager.load_card_from_json("first_place_qualification_r")
			if card2:
				DeckManager.add_card_to_deck(card2)
			set_flag("rival_true_ending_progress", 3)  # 3/3 완료
			print("[EventManager] Rival Route Branch 3 - A: SSR + R cards + True Ending Complete")

		"rival_branch3_B":
			# B. "같이 에이스가 되자"
			var card = DeckManager.load_card_from_json("eternal_partner_sr")
			if card:
				DeckManager.add_card_to_deck(card)
			var card2 = DeckManager.load_card_from_json("perfect_synergy_sr")
			if card2:
				DeckManager.add_card_to_deck(card2)
			var card3 = DeckManager.load_card_from_json("dual_leadership_r")
			if card3:
				DeckManager.add_card_to_deck(card3)
			set_flag("rival_alt_ending_progress", 3)  # 3/3 완료
			print("[EventManager] Rival Route Branch 3 - B: SR×2 + R cards + Alt Ending Complete")

		# ========== 우정 루트 ==========
		"friendship_branch2_A":
			# A. "코치 지시를 따른다"
			var card = DeckManager.load_card_from_json("absolute_trust_sr")
			if card:
				DeckManager.add_card_to_deck(card)
			var card2 = DeckManager.load_card_from_json("team_player_r")
			if card2:
				DeckManager.add_card_to_deck(card2)
			set_flag("friendship_branch2_choice", "A")
			print("[EventManager] Friendship Route Branch 2 - A: SR + R cards")

		"friendship_branch2_C":
			# C. "박민준과 상의한다"
			var card = DeckManager.load_card_from_json("democratic_leadership_sr")
			if card:
				DeckManager.add_card_to_deck(card)
			PlayerData.set_exclusive_trait("team_chemistry", 2)
			set_flag("friendship_branch2_choice", "C")
			print("[EventManager] Friendship Route Branch 2 - C: SR card + Team Chemistry Lv.2")

		# ========== 멘토 루트 ==========
		"mentor_branch1_A":
			# A. "코치님의 축구 인생을 듣고 싶습니다"
			var card = DeckManager.load_card_from_json("coach_kim_ssr")
			if card:
				DeckManager.add_card_to_deck(card)
			PlayerData.set_exclusive_trait("tactical_understanding", 1)
			# Determination 증가
			if PlayerData:
				var current_det = PlayerData.get_stat("mental", "determination")
				PlayerData.set_stat("mental", "determination", current_det + 20)
			set_flag("mentor_branch1_choice", "A")
			print("[EventManager] Mentor Route Branch 1 - A: SSR card + Tactical Understanding + Determination +20")

		# ========== 캡틴 루트 ==========
		"captain_branch2_C":
			# C. "중재한다 - 둘 다 틀렸어"
			var card = DeckManager.load_card_from_json("captain_authority_ssr")
			if card:
				DeckManager.add_card_to_deck(card)
			PlayerData.set_exclusive_trait("leadership", 2)
			# Leadership 스탯 증가
			if PlayerData:
				var current_leadership = PlayerData.get_stat("mental", "leadership")
				PlayerData.set_stat("mental", "leadership", current_leadership + 25)
			set_flag("captain_branch2_choice", "C")
			print("[EventManager] Captain Route Branch 2 - C: SSR card + Leadership Lv.2 + Leadership +25")

		# ========== 수호자 루트 ==========
		"guardian_branch1_B":
			# B. "내가 막아줄게. 너는 GK해"
			var card = DeckManager.load_card_from_json("guardian_combo_ssr")
			if card:
				DeckManager.add_card_to_deck(card)
			PlayerData.set_exclusive_trait("iron_defense", 1)
			set_flag("guardian_combo_unlocked", true)
			set_flag("guardian_branch1_choice", "B")
			print("[EventManager] Guardian Route Branch 1 - B: SSR card + Iron Defense Lv.1 + Combo unlocked")

		_:
			push_warning("[EventManager] Unknown branch reward key: %s" % key)


func get_branch_choice(route: String, branch_id: int) -> String:
	"""
	특정 분기점에서 선택한 선택지 반환
	@param route: 루트 ID
	@param branch_id: 분기점 번호
	@return: 선택지 ("A", "B", "C", 또는 "" if 미선택)
	"""
	var flag_key = "%s_branch%d_choice" % [route, branch_id]
	return get_flag(flag_key) if get_flag(flag_key) else ""


# ========== Phase 3: Choice History & Impact System ==========

## Choice history storage (for ChoiceLogScreen)
var choice_history: Array[Dictionary] = []

signal choice_made(choice_data: Dictionary, impacts: Array[Dictionary])


func process_choice_with_feedback(route: String, branch_id: int, choice: String, choice_text: String) -> void:
	"""
	선택지 처리 + 즉각 피드백 팝업 표시
	Phase 2의 process_branch_reward() + Phase 3 피드백 시스템 통합

	@param route: 루트 ID ("rival", "friendship", etc.)
	@param branch_id: 분기점 번호 (1, 2, 3)
	@param choice: 선택지 ("A", "B", "C")
	@param choice_text: 선택지 텍스트 (예: "정정당당하게 경쟁하자")
	"""
	print("[EventManager] Processing choice with feedback: %s branch%d choice %s" % [route, branch_id, choice])

	# 1. Calculate impacts BEFORE applying effects
	var impacts = _calculate_choice_impacts(route, branch_id, choice)

	# 2. Apply effects (Phase 2 process_branch_reward)
	process_branch_reward(route, branch_id, choice)

	# 3. Record choice in history
	var choice_record = {
		"week": GameManager.current_week if GameManager else 0,
		"route": route,
		"branch_id": branch_id,
		"choice": choice,
		"choice_text": choice_text,
		"immediate_impacts": impacts,
		"long_term_impacts": _calculate_long_term_impacts(route, branch_id, choice),
		"timestamp": Time.get_unix_time_from_system()
	}
	choice_history.append(choice_record)

	# Phase 24: Log decision to DecisionTracker
	if DecisionTracker:
		# Build alternatives list (typically A/B or A/B/C for story events)
		var alternatives = []
		var all_choices = ["A", "B", "C"]
		for c in all_choices:
			if c != choice:
				alternatives.append("선택지 %s" % c)

		# Log the event decision
		DecisionTracker.log_decision(
			"event",
			choice_text,
			alternatives,
			{
				"route": route,
				"branch_id": branch_id,
				"week": GameManager.current_week if GameManager else 0,
				"choice_letter": choice
			},
			{
				"immediate_impacts": impacts.duplicate(true),
				"long_term_impacts": choice_record.long_term_impacts.duplicate(true)
			}
		)

	# 4. Emit signal with impacts
	choice_made.emit(choice_record, impacts)

	# 5. Show impact popup
	_show_impact_popup(impacts)

	print("[EventManager] Choice processed: %d total choices in history" % choice_history.size())


func _calculate_choice_impacts(route: String, branch_id: int, choice: String) -> Array[Dictionary]:
	"""
	선택지의 즉각 효과 계산 (팝업에 표시할 내용)

	@return: Array of impact dictionaries:
		[
			{"type": "affection", "character": "kang_taeyang", "value": 20, "from": 60, "to": 80},
			{"type": "trait", "trait_name": "rival_awakening", "level": 1},
			{"type": "card", "card_id": "kang_taeyang_ssr_rival"},
			{"type": "ending", "ending_id": "rival_true_ending", "progress": 1, "total": 3}
		]
	"""
	var impacts: Array[Dictionary] = []
	var key = "%s_branch%d_%s" % [route, branch_id, choice]

	# Match against known branch rewards (from Phase 2)
	match key:
		"rival_branch2_A":
			# SSR card
			impacts.append({"type": "card", "card_id": "kang_taeyang_ssr_rival"})
			# Exclusive trait
			impacts.append({"type": "trait", "trait_name": "rival_awakening", "level": 1})
			# Ending progress
			impacts.append({"type": "ending", "ending_id": "rival_true_ending", "progress": 1, "total": 3})
			# Affection (if tracked)
			if character_affection.has("kang_taeyang"):
				var old_val = character_affection["kang_taeyang"]
				var new_val = mini(old_val + 20, 100)
				impacts.append(
					{"type": "affection", "character": "kang_taeyang", "value": 20, "from": old_val, "to": new_val}
				)

		"rival_branch2_B":
			impacts.append({"type": "card", "card_id": "dual_ace_sr_tactical"})
			impacts.append({"type": "stat", "stat_name": "Teamwork", "value": 15})
			impacts.append({"type": "ending", "ending_id": "rival_alt_ending", "progress": 1, "total": 3})

		"rival_branch3_A":
			impacts.append({"type": "card", "card_id": "absolute_ace_ssr"})
			impacts.append({"type": "card", "card_id": "first_place_qualification_r"})
			impacts.append({"type": "trait", "trait_name": "rival_awakening", "level": 3})
			impacts.append({"type": "ending", "ending_id": "rival_true_ending", "progress": 3, "total": 3})

		"rival_branch3_B":
			impacts.append({"type": "card", "card_id": "eternal_partner_sr"})
			impacts.append({"type": "card", "card_id": "perfect_synergy_sr"})
			impacts.append({"type": "card", "card_id": "dual_leadership_r"})
			impacts.append({"type": "ending", "ending_id": "rival_alt_ending", "progress": 3, "total": 3})

		"friendship_branch2_A":
			impacts.append({"type": "card", "card_id": "absolute_trust_sr"})
			impacts.append({"type": "card", "card_id": "team_player_r"})
			if character_affection.has("park_minjun"):
				var old_val = character_affection["park_minjun"]
				var new_val = mini(old_val + 15, 100)
				impacts.append(
					{"type": "affection", "character": "park_minjun", "value": 15, "from": old_val, "to": new_val}
				)

		"friendship_branch2_C":
			impacts.append({"type": "card", "card_id": "democratic_leadership_sr"})
			impacts.append({"type": "trait", "trait_name": "team_chemistry", "level": 2})

		"mentor_branch1_A":
			impacts.append({"type": "card", "card_id": "coach_kim_ssr"})
			impacts.append({"type": "trait", "trait_name": "tactical_understanding", "level": 1})
			impacts.append({"type": "stat", "stat_name": "Determination", "value": 20})

		"captain_branch2_C":
			impacts.append({"type": "card", "card_id": "captain_authority_ssr"})
			impacts.append({"type": "trait", "trait_name": "leadership", "level": 2})
			impacts.append({"type": "stat", "stat_name": "Leadership", "value": 25})

		"guardian_branch1_B":
			impacts.append({"type": "card", "card_id": "guardian_combo_ssr"})
			impacts.append({"type": "trait", "trait_name": "iron_defense", "level": 1})

	return impacts


func _calculate_long_term_impacts(route: String, branch_id: int, choice: String) -> Array[String]:
	"""
	선택지의 장기 영향 계산 (텍스트 설명)

	@return: Array of long-term impact descriptions
	"""
	var long_term: Array[String] = []
	var key = "%s_branch%d_%s" % [route, branch_id, choice]

	match key:
		"rival_branch2_A":
			long_term.append("Week 80: '질투의 폭발' 이벤트 잠금")
			long_term.append("Week 95: '진정한 경쟁자' 이벤트 언락")
			long_term.append("True Ending 루트 진입")

		"rival_branch2_B":
			long_term.append("Week 85: '듀얼 에이스' 이벤트 언락")
			long_term.append("Alternative Ending 루트 진입")

		"friendship_branch2_A":
			long_term.append("코치와의 신뢰 관계 형성")
			long_term.append("팀 보너스 +15% 지속 효과")

		"mentor_branch1_A":
			long_term.append("김철수 코치 특별 훈련 언락")
			long_term.append("전술 이해도 영구 향상")

	return long_term


func _show_impact_popup(impacts: Array[Dictionary]) -> void:
	"""
	ChoiceImpactPopup을 화면에 표시

	@param impacts: Impact data array
	"""
	if impacts.is_empty():
		print("[EventManager] No impacts to show")
		return

	# Load popup scene
	var popup_scene = load("res://scenes/ui/ChoiceImpactPopup.tscn")
	if not popup_scene:
		push_error("[EventManager] ChoiceImpactPopup scene not found!")
		return

	# Instantiate and add to scene tree
	var popup = popup_scene.instantiate()

	# Add to root (above all other UI)
	var root = get_tree().root
	root.add_child(popup)

	# Center popup
	if popup is Control:
		popup.position = Vector2((root.size.x - popup.size.x) / 2, (root.size.y - popup.size.y) / 2)

	# Show impacts
	popup.show_impacts(impacts)

	print("[EventManager] Impact popup displayed with %d impacts" % impacts.size())


func get_choice_history() -> Array[Dictionary]:
	"""
	Get all choice history for ChoiceLogScreen

	@return: Array of choice records (newest first)
	"""
	var history = choice_history.duplicate()
	history.reverse()  # Newest first
	return history


func clear_choice_history() -> void:
	"""Clear all choice history (for new game)"""
	choice_history.clear()
	print("[EventManager] Choice history cleared")


# ========== Phase 3.4: Tension System ==========

## Tension curve data (Week → Tension level 1-10)
var event_tension_curve: Dictionary = {
	1: 2, 10: 3, 20: 4, 40: 5, 60: 6, 75: 7, 85: 8, 100: 7, 120: 8, 135: 10, 140: 10, 156: 4  # 입단  # 초반 적응  # 첫 경기  # 1차 대결  # Low Point 시작  # Low Point 최고조 + B-Team 평가  # Midpoint (반전)  # 약간 하강  # A-Team 도전 시작  # Climax (A-Team 평가)  # Climax 지속  # 해소 (엔딩)
}


func get_current_tension(week: int = -1) -> int:
	"""
	현재 Week의 텐션 레벨 반환 (1-10)

	@param week: 텐션을 계산할 Week (기본값: GameManager.current_week)
	@return: 텐션 레벨 (1-10)

	텐션 곡선:
	- Week 1-40: 2→5 (완만한 상승)
	- Week 40-75: 5→7 (Low Points)
	- Week 75-85: 7→8 (Midpoint)
	- Week 85-135: 8→10 (Climax 준비)
	- Week 135-140: 10 (Climax)
	- Week 140-156: 10→4 (해소)
	"""
	# Use GameManager week if not specified
	if week < 0:
		week = GameManager.current_week if GameManager else 1

	# Get sorted keys
	var keys = event_tension_curve.keys()
	keys.sort()

	# If week is before first key, return first value
	if week <= keys[0]:
		return event_tension_curve[keys[0]]

	# If week is after last key, return last value
	if week >= keys[-1]:
		return event_tension_curve[keys[-1]]

	# Linear interpolation between two closest keys
	for i in range(keys.size() - 1):
		if week >= keys[i] and week < keys[i + 1]:
			var week_start = keys[i]
			var week_end = keys[i + 1]
			var tension_start = event_tension_curve[week_start]
			var tension_end = event_tension_curve[week_end]

			# Linear interpolation
			var t = float(week - week_start) / (week_end - week_start)
			var tension = lerp(float(tension_start), float(tension_end), t)

			return int(round(tension))

	# Fallback
	return 5


func should_trigger_random_event() -> bool:
	"""
	텐션에 따라 랜덤 이벤트 발생 확률 조절

	@return: True if random event should trigger

	확률 공식:
	- Tension 1-3: 10% (저텐션, 이벤트 적음)
	- Tension 4-6: 25% (중텐션)
	- Tension 7-8: 40% (고텐션)
	- Tension 9-10: 60% (Climax, 이벤트 많음)
	"""
	var tension = get_current_tension()
	var probability = 0.0

	if tension <= 3:
		probability = 0.10
	elif tension <= 6:
		probability = 0.25
	elif tension <= 8:
		probability = 0.40
	else:  # 9-10
		probability = 0.60

	var roll = randf()
	var should_trigger = roll < probability

	if should_trigger:
		print("[EventManager] Random event triggered (Tension %d, Roll %.2f < %.2f)" % [tension, roll, probability])

	return should_trigger


func get_tension_description(tension: int) -> String:
	"""
	텐션 레벨에 대한 설명 반환

	@param tension: 텐션 레벨 (1-10)
	@return: 한글 설명
	"""
	match tension:
		1, 2:
			return "평온 (새로운 시작)"
		3, 4:
			return "긴장 (도전 시작)"
		5, 6:
			return "갈등 (시련 다가옴)"
		7, 8:
			return "위기 (고비를 넘어야)"
		9, 10:
			return "절정 (모든 것을 건 순간)"
		_:
			return "알 수 없음"


func get_event_density(week_start: int, week_end: int) -> float:
	"""
	특정 Week 구간의 이벤트 밀도 측정 (디버그용)

	@param week_start: 시작 Week
	@param week_end: 종료 Week
	@return: 이벤트 밀도 (events per week)

	사용 예:
	- get_event_density(1, 40)  # Act 1 밀도
	- get_event_density(41, 120)  # Act 2 밀도
	"""
	# TODO: 실제 이벤트 데이터와 연동
	# 현재는 triggered_events에서 카운트
	var events_in_range = 0

	for event_id in triggered_events:
		# 이벤트 ID에서 Week 추출 (예: "rival_week75_event1" → 75)
		var parts = event_id.split("week")
		if parts.size() >= 2:
			var week_str = parts[1].split("_")[0]
			var event_week = int(week_str)

			if event_week >= week_start and event_week <= week_end:
				events_in_range += 1

	var weeks = week_end - week_start + 1
	var density = float(events_in_range) / weeks

	print(
		(
			"[EventManager] Event density (Week %d-%d): %.2f events/week (%d events)"
			% [week_start, week_end, density, events_in_range]
		)
	)

	return density


func get_tension_curve_data() -> Array[Dictionary]:
	"""
	텐션 곡선 데이터 반환 (UI 그래프용)

	@return: Array of {week: int, tension: int} dictionaries
	"""
	var curve_data: Array[Dictionary] = []

	# Sample every 5 weeks
	for week in range(1, 157, 5):
		curve_data.append({"week": week, "tension": get_current_tension(week)})

	return curve_data


# ========== Phase 3 Integration: JSON Event Loading ==========


func _load_arc_file(file_path: String) -> int:
	"""
	Load character arc JSON file and add events to cache

	@param file_path: Path to JSON file
	@return: Number of events loaded
	"""
	if not FileAccess.file_exists(file_path):
		push_warning("[EventManager] Arc file not found: %s" % file_path)
		return 0

	var file = FileAccess.open(file_path, FileAccess.READ)
	if not file:
		push_error("[EventManager] Failed to open arc file: %s" % file_path)
		return 0

	var json_text = file.get_as_text()
	file.close()

	var json = JSON.new()
	var error = json.parse(json_text)
	if error != OK:
		push_error("[EventManager] JSON parse error in %s: %s" % [file_path, json.get_error_message()])
		return 0

	var arc_data = json.data
	if not arc_data is Dictionary:
		push_error("[EventManager] Invalid arc data structure in %s" % file_path)
		return 0

	# Extract events array
	var events = arc_data.get("events", [])
	var route_id = arc_data.get("route_id", "unknown")
	var character_id = arc_data.get("character_id", "unknown")

	var count = 0
	for event_data in events:
		if event_data is Dictionary:
			var event_id = event_data.get("event_id", "")
			if event_id != "":
				# Add metadata
				event_data["route"] = route_id
				event_data["character"] = character_id

				# Store in cache
				event_cache[event_id] = event_data
				count += 1

	print("[EventManager] Loaded %d events from %s (%s - %s)" % [count, file_path, route_id, character_id])
	return count


func get_event_by_week(week: int, _timing: String = "pre") -> Dictionary:
	"""
	Get event for specific week

	@param week: Week number (1-156)
	@param timing: "pre" or "post"
	@return: Event data dictionary (empty if no event)
	"""
	# Find events matching this week
	var matching_events = []

	for event_id in event_cache:
		var event_data = event_cache[event_id]
		var event_week = event_data.get("week", 0)

		if event_week == week:
			# Check if conditions are met
			if _check_event_conditions(event_data):
				matching_events.append(event_data)

	# If multiple events match, prioritize by type
	if matching_events.size() > 0:
		matching_events.sort_custom(_sort_events_by_priority)
		return matching_events[0]

	return {}


func _check_event_conditions(event_data: Dictionary) -> bool:
	"""
	Check if event conditions are met

	@param event_data: Event data from cache
	@return: True if all conditions met
	"""
	var condition = event_data.get("condition", {})

	if condition.is_empty():
		return true  # No conditions, always available

	# Check flags
	var required_flags = condition.get("flags", [])
	for flag in required_flags:
		if not get_flag(flag):
			return false

	# Check affection (if specified)
	var min_affection = condition.get("min_affection", -1)
	if min_affection >= 0:
		var character = event_data.get("character", "")
		var current_affection = character_affection.get(character, 50)
		if current_affection < min_affection:
			return false

	# Check CA (if specified)
	var min_ca = condition.get("min_ca", -1)
	if min_ca >= 0:
		if PlayerData and PlayerData.current_ability < min_ca:
			return false

	return true


func _sort_events_by_priority(a: Dictionary, b: Dictionary) -> bool:
	"""
	Sort events by priority (for when multiple events match same week)

	Priority order:
	1. critical_choice (highest)
	2. low_point
	3. minigame
	4. fall_climax
	5. rise_start
	6. resolution
	7. fall_start (lowest)
	"""
	var priority_order = {
		"critical_choice": 7,
		"low_point": 6,
		"minigame": 5,
		"fall_climax": 4,
		"rise_start": 3,
		"resolution": 2,
		"fall_start": 1,
		"fall_middle": 1
	}

	var type_a = a.get("type", "")
	var type_b = b.get("type", "")

	var priority_a = priority_order.get(type_a, 0)
	var priority_b = priority_order.get(type_b, 0)

	return priority_a > priority_b


func _apply_event_effects(effects: Dictionary) -> void:
	"""
	Apply event effects (affection, flags, stats)

	@param effects: Effects dictionary from event data
	"""
	# Affection change
	var affection_change = effects.get("affection_change", 0)
	if affection_change != 0 and active_event.has("character"):
		var character = active_event["character"]
		modify_affection(character, affection_change)

	# Flags
	var flags = effects.get("flags", [])
	for flag in flags:
		set_flag(flag, true)

	# Stats
	var stats = effects.get("stats", {})
	if PlayerData:
		for stat_name in stats:
			var _stat_value = stats[stat_name]
			# TODO: Apply stat change to PlayerData

	# Special training unlocked
	var special_training = effects.get("special_training_unlocked", "")
	if special_training != "":
		set_flag("training_" + special_training, true)
		print("[EventManager] Special training unlocked: %s" % special_training)


func process_event_choice(choice_id: String) -> void:
	"""
	Process player's choice in active event

	@param choice_id: Choice ID ("A", "B", "C", etc.)
	"""
	if active_event.is_empty():
		push_warning("[EventManager] No active event to process choice")
		return

	var choices = active_event.get("choices", [])
	var selected_choice = null

	for choice in choices:
		if choice.get("choice_id", "") == choice_id:
			selected_choice = choice
			break

	if not selected_choice:
		push_warning("[EventManager] Choice %s not found in active event" % choice_id)
		return

	print("[EventManager] Player selected choice: %s" % choice_id)

	# Apply choice effects
	var effects = selected_choice.get("effects", {})
	_apply_event_effects(effects)

	# If this is a branch point, use process_choice_with_feedback (Phase 3.5)
	var route = active_event.get("route", "")
	var event_type = active_event.get("type", "")

	if event_type in ["low_point", "critical_choice"]:
		var choice_text = selected_choice.get("text", "")
		# Extract branch_id from event_id (e.g., "rival_week75_bench_lowpoint" → 75)
		var event_id = active_event.get("event_id", "")
		var week = active_event.get("week", 0)

		# Trigger feedback system
		process_choice_with_feedback(route, week, choice_id, choice_text)

	# Queue next event (if specified)
	var next_event_id = effects.get("next_event", "")
	if next_event_id != "":
		set_flag("queued_event_" + next_event_id, true)


func get_queued_events() -> Array[Dictionary]:
	"""
	Get events queued by previous choices

	@return: Array of queued event data
	"""
	var queued: Array[Dictionary] = []

	for flag_key in event_flags:
		if flag_key.begins_with("queued_event_"):
			var event_id = flag_key.replace("queued_event_", "")

			if event_cache.has(event_id):
				var event_data = event_cache[event_id]

				# Check if conditions are met
				if _check_event_conditions(event_data):
					queued.append(event_data)
					# Remove flag after queuing
					event_flags.erase(flag_key)

	return queued
