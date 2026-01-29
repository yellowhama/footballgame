extends Node
# Removed class_name to avoid autoload singleton conflict

## Phase 6.2: Idle/AFK Rewards System
## 게임 미접속 시간에 비례한 보상 누적. 복귀 시 팝업으로 수령.

signal idle_rewards_available(rewards: Dictionary)
signal idle_rewards_claimed(rewards: Dictionary)
signal idle_status_updated(elapsed_seconds: int)

## Constants - 보상 레이트 (시간당)
const IDLE_REWARD_RATE: Dictionary = {"gold": 10, "coach_exp": 5, "stamina": 2, "gacha_token": 0.5}  # 시간당 골드  # 시간당 코치 경험치  # 시간당 스태미나 회복  # 시간당 가챠 토큰 (2시간에 1개)

const MAX_IDLE_HOURS: float = 24.0  # 최대 누적 시간 (24시간)
const MIN_IDLE_MINUTES: float = 5.0  # 최소 방치 시간 (5분)
const SAVE_KEY: String = "idle_reward_timestamp"

## Stage-based multipliers (스테이지 레벨에 따른 보상 배율)
const STAGE_MULTIPLIERS: Dictionary = {0: 1.0, 10: 1.2, 20: 1.4, 30: 1.6, 50: 2.0, 70: 2.5, 100: 3.0}  # 기본  # 스테이지 10 이상  # 스테이지 20 이상  # 스테이지 30 이상  # 스테이지 50 이상  # 스테이지 70 이상  # 스테이지 100 이상

## State
var last_active_timestamp: int = 0
var pending_rewards: Dictionary = {}
var highest_stage_cleared: int = 0
var _auto_save_timer: Timer = null


func _ready() -> void:
	load_timestamp()
	check_idle_rewards()
	_setup_auto_save()
	print("[IdleRewardManager] Initialized (last_active: %d)" % last_active_timestamp)


func _setup_auto_save() -> void:
	"""주기적 타임스탬프 저장 설정."""
	_auto_save_timer = Timer.new()
	_auto_save_timer.wait_time = 60.0  # 1분마다 저장
	_auto_save_timer.autostart = true
	_auto_save_timer.timeout.connect(_on_auto_save_timeout)
	add_child(_auto_save_timer)


func _on_auto_save_timeout() -> void:
	"""자동 저장 타이머 콜백."""
	save_timestamp()


func _notification(what: int) -> void:
	if what == NOTIFICATION_WM_CLOSE_REQUEST:
		save_timestamp()
	elif what == NOTIFICATION_WM_GO_BACK_REQUEST:
		save_timestamp()
	elif what == NOTIFICATION_APPLICATION_FOCUS_OUT:
		save_timestamp()


## ========== Timestamp Management ==========


func save_timestamp() -> void:
	"""현재 시간을 타임스탬프로 저장."""
	last_active_timestamp = int(Time.get_unix_time_from_system())

	var save_manager = get_node_or_null("/root/SaveManager")
	if save_manager and save_manager.has_method("save_meta_data"):
		save_manager.save_meta_data(SAVE_KEY, last_active_timestamp)
	else:
		# 폴백: 직접 파일 저장
		var config = ConfigFile.new()
		config.set_value("idle", "last_active", last_active_timestamp)
		config.set_value("idle", "highest_stage", highest_stage_cleared)
		var err = config.save("user://idle_rewards.cfg")
		if err != OK:
			push_warning("[IdleRewardManager] Failed to save timestamp: %d" % err)


func load_timestamp() -> void:
	"""저장된 타임스탬프 로드."""
	var save_manager = get_node_or_null("/root/SaveManager")
	if save_manager and save_manager.has_method("load_meta_data"):
		last_active_timestamp = save_manager.load_meta_data(SAVE_KEY, 0)
	else:
		# 폴백: 직접 파일 로드
		var config = ConfigFile.new()
		var err = config.load("user://idle_rewards.cfg")
		if err == OK:
			last_active_timestamp = config.get_value("idle", "last_active", 0)
			highest_stage_cleared = config.get_value("idle", "highest_stage", 0)
		else:
			last_active_timestamp = 0

	if last_active_timestamp == 0:
		last_active_timestamp = int(Time.get_unix_time_from_system())
		save_timestamp()


## ========== Reward Calculation ==========


func check_idle_rewards() -> void:
	"""방치 보상 확인 및 알림."""
	if last_active_timestamp == 0:
		last_active_timestamp = int(Time.get_unix_time_from_system())
		return

	var now: int = int(Time.get_unix_time_from_system())
	var elapsed: int = now - last_active_timestamp

	idle_status_updated.emit(elapsed)

	var rewards: Dictionary = calculate_idle_rewards(elapsed)
	if not rewards.is_empty():
		pending_rewards = rewards
		idle_rewards_available.emit(rewards)
		print("[IdleRewardManager] Idle rewards available: %s" % str(rewards))


func calculate_idle_rewards(elapsed_seconds: int) -> Dictionary:
	"""방치 보상 계산.

	Args:
		elapsed_seconds: 경과 시간 (초)

	Returns:
		보상 Dictionary or empty if below minimum
	"""
	var minutes: float = elapsed_seconds / 60.0
	if minutes < MIN_IDLE_MINUTES:
		return {}

	var hours: float = minf(elapsed_seconds / 3600.0, MAX_IDLE_HOURS)
	var multiplier: float = _get_stage_multiplier()

	var rewards: Dictionary = {
		"gold": int(hours * IDLE_REWARD_RATE.gold * multiplier),
		"coach_exp": int(hours * IDLE_REWARD_RATE.coach_exp * multiplier),
		"stamina": int(hours * IDLE_REWARD_RATE.stamina * multiplier),
		"gacha_token": int(hours * IDLE_REWARD_RATE.gacha_token * multiplier),
		"hours_calculated": hours,
		"multiplier": multiplier,
		"elapsed_seconds": elapsed_seconds
	}

	# 0인 보상 제거
	var cleaned: Dictionary = {}
	for key in rewards:
		if rewards[key] != 0:
			cleaned[key] = rewards[key]

	return cleaned


func _get_stage_multiplier() -> float:
	"""스테이지 기반 보상 배율 계산."""
	var multiplier: float = 1.0
	for stage_threshold in STAGE_MULTIPLIERS:
		if highest_stage_cleared >= stage_threshold:
			multiplier = STAGE_MULTIPLIERS[stage_threshold]
	return multiplier


## ========== Reward Claiming ==========


func claim_rewards(rewards: Dictionary = {}) -> bool:
	"""보상 수령.

	Args:
		rewards: 수령할 보상 (비어있으면 pending_rewards 사용)

	Returns:
		수령 성공 여부
	"""
	var to_claim: Dictionary = rewards if not rewards.is_empty() else pending_rewards
	if to_claim.is_empty():
		return false

	# 실제 보상 지급
	var game_manager = get_node_or_null("/root/GameManager")
	if game_manager:
		if game_manager.has_method("add_gold"):
			game_manager.add_gold(to_claim.get("gold", 0))
		if game_manager.has_method("add_stamina"):
			game_manager.add_stamina(to_claim.get("stamina", 0))
		if game_manager.has_method("add_gacha_token"):
			game_manager.add_gacha_token(to_claim.get("gacha_token", 0))

	# 코치 경험치 지급
	var coach_system = get_node_or_null("/root/CoachCardSystem")
	if coach_system and coach_system.has_method("add_exp_to_all"):
		coach_system.add_exp_to_all(to_claim.get("coach_exp", 0))

	idle_rewards_claimed.emit(to_claim)
	print("[IdleRewardManager] Claimed rewards: %s" % str(to_claim))

	# 상태 리셋
	pending_rewards.clear()
	last_active_timestamp = int(Time.get_unix_time_from_system())
	save_timestamp()

	return true


func has_pending_rewards() -> bool:
	"""대기 중인 보상 여부."""
	return not pending_rewards.is_empty()


func get_pending_rewards() -> Dictionary:
	"""대기 중인 보상 반환."""
	return pending_rewards.duplicate()


## ========== Stage Progress Integration ==========


func update_highest_stage(stage: int) -> void:
	"""최고 클리어 스테이지 업데이트.

	Args:
		stage: 새로 클리어한 스테이지 번호
	"""
	if stage > highest_stage_cleared:
		highest_stage_cleared = stage
		save_timestamp()
		print(
			(
				"[IdleRewardManager] Highest stage updated: %d (multiplier: %.1fx)"
				% [highest_stage_cleared, _get_stage_multiplier()]
			)
		)


func get_current_multiplier() -> float:
	"""현재 보상 배율."""
	return _get_stage_multiplier()


## ========== Utility ==========


func format_elapsed_time(seconds: int) -> String:
	"""경과 시간을 읽기 좋은 형식으로 포맷."""
	var hours: int = seconds / 3600
	var minutes: int = (seconds % 3600) / 60

	if hours > 0:
		return "%d시간 %d분" % [hours, minutes]
	else:
		return "%d분" % minutes


func get_reward_preview(hours: float) -> Dictionary:
	"""특정 시간에 대한 보상 미리보기.

	Args:
		hours: 미리보기 시간 (시)

	Returns:
		예상 보상 Dictionary
	"""
	var seconds: int = int(hours * 3600)
	return calculate_idle_rewards(seconds)


## ========== Debug ==========


func debug_simulate_idle(hours: float) -> void:
	"""디버그: 특정 시간 방치 시뮬레이션."""
	var fake_elapsed: int = int(hours * 3600)
	var rewards: Dictionary = calculate_idle_rewards(fake_elapsed)
	print("[IdleRewardManager] DEBUG - Simulated %dh idle: %s" % [int(hours), str(rewards)])
	if not rewards.is_empty():
		pending_rewards = rewards
		idle_rewards_available.emit(rewards)


func debug_print_status() -> void:
	"""디버그: 현재 상태 출력."""
	print("\n" + "=".repeat(50))
	print("[IdleRewardManager] Status")
	print("=".repeat(50))
	print("Last active: %d" % last_active_timestamp)
	print("Highest stage: %d" % highest_stage_cleared)
	print("Current multiplier: %.1fx" % _get_stage_multiplier())
	print("Pending rewards: %s" % str(pending_rewards))
	print("=".repeat(50) + "\n")
