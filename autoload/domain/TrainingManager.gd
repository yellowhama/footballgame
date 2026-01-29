extends Node
## TrainingManager - 훈련 시스템 로직 관리
## Phase 8 Implementation: Training execution and attribute updates

# Signal definitions
signal training_started(event: Dictionary)
signal training_completed(event: Dictionary)
signal training_finished(result: Dictionary)  # Emitted after all training logic is done
signal training_failed(event: Dictionary)
signal rest_activity_completed(result: Dictionary)
signal go_out_activity_completed(result: Dictionary)
signal player_attributes_changed(changes: Dictionary)

const TRAINING_MODES := ["team", "personal", "special"]
const TRAINING_INTENSITY_PRESETS := {
	"light": {"label": "����", "growth_multiplier": 0.8, "fatigue_multiplier": 0.7, "api_value": "Light"},
	"normal": {"label": "���", "growth_multiplier": 1.0, "fatigue_multiplier": 1.0, "api_value": "Normal"},
	"intense": {"label": "���", "growth_multiplier": 1.3, "fatigue_multiplier": 1.25, "api_value": "Intensive"}
}

# Training program definitions
const SPECIAL_TRAINING_DATA_PATH := "res://data/special_trainings.json"

var TRAINING_PROGRAMS = {
	# General training programs (6)
	"shooting":
	{
		"id": "shooting",
		"name": "슈팅 훈련",
		"type": "technical",
		"duration": 60,
		"attributes": {"finishing": 2, "long_shots": 1},
		"condition_cost": 8,  # Using ConditionSystem percentage cost
		"description": "골 결정력과 중거리 슈팅 능력 향상"
	},
	"passing":
	{
		"id": "passing",
		"name": "패스 훈련",
		"type": "technical",
		"duration": 60,
		"attributes": {"passing": 2, "crossing": 1},
		"condition_cost": 6,
		"description": "패스 정확도와 크로스 능력 향상"
	},
	"dribbling":
	{
		"id": "dribbling",
		"name": "드리블 훈련",
		"type": "technical",
		"duration": 60,
		"attributes": {"dribbling": 2, "agility": 1},
		"condition_cost": 10,
		"description": "드리블 기술과 민첩성 향상"
	},
	"physical":
	{
		"id": "physical",
		"name": "체력 훈련",
		"type": "physical",
		"duration": 90,
		"attributes": {"stamina": 3, "strength": 2},
		"condition_cost": 15,
		"description": "지구력과 신체 능력 향상"
	},
	"tactical":
	{
		"id": "tactical",
		"name": "전술 훈련",
		"type": "tactical",
		"duration": 75,
		"attributes": {"vision": 2, "positioning": 2},
		"condition_cost": 5,
		"description": "시야와 포지셔닝 능력 향상"
	},
	"defending":
	{
		"id": "defending",
		"name": "수비 훈련",
		"type": "defensive",
		"duration": 60,
		"attributes": {"tackling": 2, "marking": 1},
		"condition_cost": 8,
		"description": "태클과 마크 능력 향상"
	},
	# Special training programs
	"special_speed_camp":
	{
		"id": "special_speed_camp",
		"name": "Speed Focus Camp",
		"type": "special",
		"duration": 120,
		"attributes": {"acceleration": 3, "agility": 2},
		"condition_cost": 18,
		"description": "Short burst camp focused on explosive sprint work."
	},
	"special_focus_retreat":
	{
		"id": "special_focus_retreat",
		"name": "Tactics Focus Retreat",
		"type": "special",
		"duration": 110,
		"attributes": {"vision": 3, "composure": 2},
		"condition_cost": 16,
		"description": "An off-site retreat to sharpen vision and composure with the coaching staff."
	},
	"special_recovery_lab":
	{
		"id": "special_recovery_lab",
		"name": "Recovery Care Lab",
		"type": "special",
		"duration": 80,
		"attributes": {"stamina": 2, "balance": 1},
		"condition_cost": 6,
		"description": "Sports science led recovery program for long-term stamina care."
	},
	# Personal focused training programs (15)
	"shooting_precision":
	{
		"id": "shooting_precision",
		"name": "정밀 슈팅",
		"type": "technical",
		"duration": 45,
		"attributes": {"finishing": 3},
		"condition_cost": 7,
		"description": "골 결정력 집중 향상"
	},
	"passing_accuracy":
	{
		"id": "passing_accuracy",
		"name": "패스 정확도",
		"type": "technical",
		"duration": 45,
		"attributes": {"passing": 3},
		"condition_cost": 5,
		"description": "패스 정확도 집중 향상"
	},
	"dribbling_control":
	{
		"id": "dribbling_control",
		"name": "드리블 컨트롤",
		"type": "technical",
		"duration": 50,
		"attributes": {"dribbling": 3},
		"condition_cost": 8,
		"description": "드리블 기술 집중 향상"
	},
	"crossing_practice":
	{
		"id": "crossing_practice",
		"name": "크로스 연습",
		"type": "technical",
		"duration": 40,
		"attributes": {"crossing": 3},
		"condition_cost": 6,
		"description": "크로스 능력 집중 향상"
	},
	"heading_training":
	{
		"id": "heading_training",
		"name": "헤딩 훈련",
		"type": "technical",
		"duration": 40,
		"attributes": {"heading": 3},
		"condition_cost": 7,
		"description": "헤딩 능력 집중 향상"
	},
	"tackling_drills":
	{
		"id": "tackling_drills",
		"name": "태클 드릴",
		"type": "defensive",
		"duration": 45,
		"attributes": {"tackling": 3},
		"condition_cost": 9,
		"description": "태클 기술 집중 향상"
	},
	"positioning_work":
	{
		"id": "positioning_work",
		"name": "포지셔닝 작업",
		"type": "tactical",
		"duration": 50,
		"attributes": {"positioning": 3},
		"condition_cost": 4,
		"description": "포지션 선정 능력 집중 향상"
	},
	"marking_practice":
	{
		"id": "marking_practice",
		"name": "마킹 연습",
		"type": "defensive",
		"duration": 45,
		"attributes": {"marking": 3},
		"condition_cost": 6,
		"description": "마킹 능력 집중 향상"
	},
	"interception_drills":
	{
		"id": "interception_drills",
		"name": "인터셉트 드릴",
		"type": "defensive",
		"duration": 45,
		"attributes": {"marking": 3},
		"condition_cost": 7,
		"description": "볼 차단 능력 집중 향상"
	},
	"stamina_boost":
	{
		"id": "stamina_boost",
		"name": "지구력 강화",
		"type": "physical",
		"duration": 60,
		"attributes": {"stamina": 4},
		"condition_cost": 12,
		"description": "지구력 집중 향상"
	},
	"pace_training":
	{
		"id": "pace_training",
		"name": "스피드 훈련",
		"type": "physical",
		"duration": 50,
		"attributes": {"pace": 3},
		"condition_cost": 10,
		"description": "주력 집중 향상"
	},
	"strength_conditioning":
	{
		"id": "strength_conditioning",
		"name": "근력 강화",
		"type": "physical",
		"duration": 60,
		"attributes": {"strength": 4},
		"condition_cost": 13,
		"description": "신체 강도 집중 향상"
	},
	"agility_drills":
	{
		"id": "agility_drills",
		"name": "민첩성 드릴",
		"type": "physical",
		"duration": 45,
		"attributes": {"agility": 3},
		"condition_cost": 9,
		"description": "민첩성 집중 향상"
	},
	"mental_toughness":
	{
		"id": "mental_toughness",
		"name": "멘탈 강화",
		"type": "mental",
		"duration": 60,
		"attributes": {"Mentality": 3},
		"condition_cost": 5,
		"description": "정신력 집중 향상"
	},
	"decision_making":
	{
		"id": "decision_making",
		"name": "판단력 훈련",
		"type": "mental",
		"duration": 50,
		"attributes": {"vision": 3},
		"condition_cost": 4,
		"description": "의사결정 능력 집중 향상"
	}
}

# Minimum condition percentage required to train
const MIN_CONDITION_TO_TRAIN = 40.0
const MAX_PERSONAL_TRAININGS_PER_WEEK = 3
const REST_CAUTION_PENALTY := 0.85

var personal_trainings_completed: int = 0

# Phase 9.2: Training history tracking
var training_history: Array = []  # Array of training session records
var _rest_caution_active: bool = false
var _rest_caution_info: Dictionary = {}
var _frontend_training_mode: String = "personal"
var _frontend_intensity: String = "normal"
var _last_deck_bonus_info: Dictionary = {}
var _last_deck_snapshot: Array = []


func set_frontend_training_mode(mode: String) -> void:
	if not TRAINING_MODES.has(mode):
		return
	_frontend_training_mode = mode


func set_training_intensity(intensity_id: String) -> void:
	if not TRAINING_INTENSITY_PRESETS.has(intensity_id):
		return
	_frontend_intensity = intensity_id


func get_training_intensity() -> String:
	return _frontend_intensity


func _get_intensity_config() -> Dictionary:
	return TRAINING_INTENSITY_PRESETS.get(_frontend_intensity, TRAINING_INTENSITY_PRESETS["normal"])


func _load_special_training_programs() -> void:
	if not FileAccess.file_exists(SPECIAL_TRAINING_DATA_PATH):
		return
	var file := FileAccess.open(SPECIAL_TRAINING_DATA_PATH, FileAccess.READ)
	if not file:
		push_warning("[TrainingManager] Failed to open %s" % SPECIAL_TRAINING_DATA_PATH)
		return
	var content := file.get_as_text()
	file.close()
	var parser := JSON.new()
	var parse_result := parser.parse(content)
	if parse_result != OK:
		push_warning(
			"[TrainingManager] Failed to parse %s: %s" % [SPECIAL_TRAINING_DATA_PATH, parser.get_error_message()]
		)
		return
	var entries = parser.data
	if not (entries is Array):
		push_warning("[TrainingManager] Special training data format invalid (expected Array)")
		return
	var added := 0
	for raw_entry in entries:
		if not (raw_entry is Dictionary):
			continue
		var entry: Dictionary = raw_entry
		var training_id := String(entry.get("id", "")).strip_edges()
		if training_id.is_empty():
			continue
		var program := {
			"id": training_id,
			"name": entry.get("name", training_id),
			"type": entry.get("type", "special"),
			"duration": int(entry.get("duration", 90)),
			"attributes": entry.get("attributes", {}),
			"condition_cost": int(entry.get("condition_cost", 10)),
			"description": entry.get("description", "")
		}
		if entry.has("availability"):
			program["availability"] = entry["availability"]
		if entry.has("ui_note"):
			program["ui_note"] = entry["ui_note"]
		TRAINING_PROGRAMS[training_id] = program
		added += 1
	if added > 0:
		print("[TrainingManager] Loaded %d special training programs from data file" % added)


func _snapshot_current_deck() -> Array:
	# SSOT: Rust deck (DeckBuilderScreen updates Rust deck directly; DeckManager is legacy UI-only)
	if not FootballRustEngine or not FootballRustEngine.is_ready():
		return []
	if not FootballRustEngine.has_method("deck_get_active"):
		return []

	var active := FootballRustEngine.deck_get_active()
	if not active.get("success", false):
		return []

	var deck: Dictionary = active.get("deck", {})
	if deck.is_empty():
		return []

	var slots: Dictionary = deck.get("slots", {})
	var snapshot: Array = []
	for slot_type in ["manager", "coach", "tactics"]:
		var arr: Array = slots.get(slot_type, [])
		for i in range(arr.size()):
			var card = arr[i]
			if card == null:
				continue
			var entry: Dictionary = {}
			entry["slot_type"] = slot_type
			entry["slot_index"] = i
			entry["id"] = card.get("id", "")
			entry["name"] = card.get("name", "")
			entry["rarity"] = card.get("rarity", 1)
			entry["type"] = str(card.get("type", card.get("card_type", ""))).to_lower()
			snapshot.append(entry)

	return snapshot


func _build_training_context(training_id: String, program: Dictionary, is_personal: bool) -> Dictionary:
	var base_mode := _frontend_training_mode if is_personal else "team"
	if not TRAINING_MODES.has(base_mode):
		base_mode = "personal"
	_last_deck_snapshot = _snapshot_current_deck()
	return {
		"mode": base_mode,
		"intensity": _frontend_intensity,
		"program_type": program.get("type", ""),
		"training_id": training_id,
		"deck": _last_deck_snapshot.duplicate(),
		"timestamp": Time.get_unix_time_from_system()
	}


func _build_training_event_payload(
	status: String, training_id: String, program: Dictionary, context: Dictionary, result: Dictionary
) -> Dictionary:
	var deck_bonus := _last_deck_bonus_info.duplicate(true) if _last_deck_bonus_info is Dictionary else {}
	return {
		"status": status,
		"training_id": training_id,
		"training_name": program.get("name", training_id),
		"mode": context.get("mode", "personal"),
		"intensity": context.get("intensity", _frontend_intensity),
		"program_type": program.get("type", ""),
		"description": program.get("description", ""),
		"ui_note": program.get("ui_note", ""),
		"deck_snapshot": context.get("deck", []),
		"deck_bonus": deck_bonus,
		"timestamp": context.get("timestamp", Time.get_unix_time_from_system()),
		"result": result.duplicate(true) if result is Dictionary else {}
	}


func _emit_training_failure(training_id: String, program: Dictionary, context: Dictionary, reason: String) -> void:
	var payload = _build_training_event_payload(
		"failed", training_id, program, context, {"success": false, "message": reason}
	)
	training_failed.emit(payload)


func _ready():
	_load_special_training_programs()
	print("[TrainingManager] Initialized with %d training programs" % TRAINING_PROGRAMS.size())


func get_available_trainings() -> Array:
	"""
	Get list of training programs available to player

	Returns:
		Array: Array of training program dictionaries
	"""
	# For now, return all programs
	# Future: Filter based on player level, team facilities, position, etc.
	var programs = []
	for program_id in TRAINING_PROGRAMS:
		programs.append(TRAINING_PROGRAMS[program_id])

	print("[TrainingManager] Retrieved %d available trainings" % programs.size())
	return programs


func get_training_by_id(training_id: String) -> Dictionary:
	"""
	Get training program details by ID

	Args:
		training_id: ID of the training program

	Returns:
		Dictionary: Training program details, or empty dict if not found
	"""
	return TRAINING_PROGRAMS.get(training_id, {})


## Calculate training load based on intensity
## Returns: int (15-40)
func _calculate_training_load(intensity_key: String) -> int:
	match intensity_key:
		"light":
			return 15
		"normal":
			return 25
		"intense":
			return 40
		_:
			return 25


func execute_training(training_id: String, is_personal: bool = true) -> Dictionary:
	"""
	Execute a training program

	Args:
		training_id: ID of the training program to execute

	Returns:
		Dictionary: Result with structure:
			{
				"success": bool,
				"changes": Dictionary,  # Attribute changes
				"condition_cost": float,  # Condition percentage cost
				"message": String
			}
	"""
	print("[TrainingManager] Attempting to execute training: %s" % training_id)

	var program = TRAINING_PROGRAMS.get(
		training_id,
		{
			"id": training_id,
			"name": training_id,
			"type": "unknown",
			"duration": 60,
			"attributes": {},
			"condition_cost": 0,
			"description": ""
		}
	)
	var context = _build_training_context(training_id, program, is_personal)
	var intensity_config = _get_intensity_config()
	var _is_team_mode: bool = context.get("mode", "personal") == "team"

	# Validate training program exists
	if not TRAINING_PROGRAMS.has(training_id):
		var error_msg = "훈련 프로그램을 찾을 수 없습니다: %s" % training_id
		push_error("[TrainingManager] %s" % error_msg)
		_emit_training_failure(training_id, program, context, error_msg)
		return {"success": false, "message": error_msg, "changes": {}, "condition_cost": 0}

	# Check condition (using ConditionSystem)
	if not ConditionSystem:
		push_error("[TrainingManager] ConditionSystem not found")
		var error_msg = "컨디션 시스템을 찾을 수 없습니다"
		_emit_training_failure(training_id, program, context, error_msg)
		return {"success": false, "message": error_msg, "changes": {}, "condition_cost": 0}

	var current_condition = ConditionSystem.get_condition_percentage()
	if current_condition < MIN_CONDITION_TO_TRAIN:
		var error_msg = "컨디션이 너무 낮아 훈련할 수 없습니다 (%.1f%% < %.1f%%)" % [current_condition, MIN_CONDITION_TO_TRAIN]
		push_warning("[TrainingManager] %s" % error_msg)
		_emit_training_failure(training_id, program, context, error_msg)
		return {"success": false, "message": error_msg, "changes": {}, "condition_cost": 0}
	if is_personal and personal_trainings_completed >= MAX_PERSONAL_TRAININGS_PER_WEEK:
		var limit_msg = "이번 주 개인 훈련 한도를 모두 사용했습니다 (최대 %d회)" % MAX_PERSONAL_TRAININGS_PER_WEEK
		push_warning("[TrainingManager] %s" % limit_msg)
		_emit_training_failure(training_id, program, context, limit_msg)
		return {"success": false, "message": limit_msg, "changes": {}, "condition_cost": 0}

	# Try Rust training engine first for growth calculations
	var started_payload = _build_training_event_payload("started", training_id, program, context, {})
	training_started.emit(started_payload)

	var changes = {}
	var modifier = ConditionSystem.get_training_modifier()
	var intensity_multiplier: float = float(intensity_config.get("growth_multiplier", 1.0))
	var fatigue_cost: float = float(program.condition_cost) * float(intensity_config.get("fatigue_multiplier", 1.0))
	if _rest_caution_active:
		modifier *= REST_CAUTION_PENALTY
		print("[TrainingManager] Rest caution applied (x%.2f)" % REST_CAUTION_PENALTY)
	if intensity_multiplier != 1.0:
		modifier *= intensity_multiplier
	var training_load_snapshot: Dictionary = {}
	var injury_risk_value: float = -1.0
	var needs_rest_warning: bool = false

	# Phase 2: Apply special training multiplier from CoachAffinitySystem
	if CoachAffinitySystem and CoachAffinitySystem.is_special_training_active():
		var special_multiplier = CoachAffinitySystem.get_training_multiplier()
		modifier *= special_multiplier
		print(
			(
				"[TrainingManager] 特訓発動! Training modifier: %.2f → %.2f (x%.1f)"
				% [ConditionSystem.get_training_modifier(), modifier, special_multiplier]
			)
		)

	# Apply deck bonus (SSOT: Rust/of_core::coach)
	var deck_bonus = _get_deck_training_bonus(program.type)
	if deck_bonus > 0.0:
		modifier *= (1.0 + deck_bonus)
		print(
			(
				"[TrainingManager] Deck bonus applied: %.2f → %.2f (+%.1f%%)"
				% [modifier / (1.0 + deck_bonus), modifier, deck_bonus * 100.0]
			)
		)

	# Apply week growth multiplier (Phase 1 Task 1.2)
	var week_multiplier = _get_week_growth_multiplier()
	modifier *= week_multiplier
	print(
		(
			"[TrainingManager] Week %d growth multiplier: x%.3f (Phase %s)"
			% [
				GameManager.current_week if GameManager else 1,
				week_multiplier,
				"1" if week_multiplier == 1.0 else ("2" if week_multiplier == 0.625 else "3")
			]
		)
	)

	if FootballRustEngine and FootballRustEngine.is_ready():
		# Build training request for Rust engine
		var training_request = {
			"training_id": training_id,
			"training_type": program.type,
			"target_attributes": program.attributes,
			"duration": program.duration,
			"intensity": _frontend_intensity,
			"training_load": _calculate_training_load(_frontend_intensity)
		}

		# Get player data from GlobalCharacterData
		var player_data = {}
		if GlobalCharacterData and GlobalCharacterData.has_method("save_to_dict"):
			player_data = GlobalCharacterData.save_to_dict()

			# ✅ NEW: Add growth_profile and personality for round-trip (Phase 20)
			if PlayerData:
				player_data["growth_profile"] = {
					"growth_rate": 1.0,
					"specialization": _get_player_specializations(PlayerData.position),
					"technical_multiplier": _get_technical_multiplier(PlayerData),
					"physical_multiplier": _get_physical_multiplier(PlayerData),
					"mental_multiplier": _get_mental_multiplier(PlayerData),
					"injury_prone": 0.1
				}
				player_data["personality"] = PlayerData.get_personality_dict()
		else:
			push_warning("[TrainingManager] Cannot get player data from GlobalCharacterData")

		# Build manager data with condition modifier
		var manager_data = {"condition_modifier": modifier, "training_quality": 1.0}  # Can be extended with coach quality

		# Normalize unsupported training types (e.g., 'tactical' -> 'mental')
		if typeof(training_request.get("training_type")) == TYPE_STRING:
			var tt: String = String(training_request.get("training_type")).to_lower()
			if tt == "tactical":
				training_request["training_type"] = "mental"
		# Call Rust training engine
		var result = FootballRustEngine.execute_training_json(training_request, player_data, manager_data)

		if result.get("success", false):
			var response_type = result.get("response_type", {})
			if typeof(response_type) == TYPE_DICTIONARY:
				var payload_type = str(response_type.get("type", ""))
				if payload_type == "TrainingResult":
					var applied_changes: bool = false
					var improvements = response_type.get("improved_attributes", [])
					if improvements is Array:
						for entry in improvements:
							if entry is Dictionary:
								var attr_name: String = str(entry.get("attribute", "")).strip_edges()
								if attr_name == "":
									continue
								var growth_value: float = float(entry.get("growth", 0.0))
								var delta: int = roundi(growth_value)
								if delta == 0 and growth_value > 0.0:
									delta = 1  # ensure at least 1 point for positive fractional growth
								changes[attr_name] = delta
								applied_changes = applied_changes or delta != 0
								print(
									(
										"[TrainingManager] Rust growth: %s +%.2f (calculated %+d)"
										% [attr_name, growth_value, delta]
									)
								)
					var attribute_map = response_type.get("attribute_changes", {})
					if attribute_map is Dictionary and attribute_map.size() > 0:
						for attr_name in attribute_map.keys():
							if not changes.has(attr_name):
								var growth_value: float = float(attribute_map[attr_name])
								var delta: int = roundi(growth_value)
								if delta == 0 and growth_value > 0.0:
									delta = 1
								changes[attr_name] = delta
								print(
									(
										"[TrainingManager] Rust growth(map): %s +%.2f (calculated %+d)"
										% [attr_name, growth_value, delta]
									)
								)
								applied_changes = applied_changes or delta != 0

					if not applied_changes:
						print(
							"[TrainingManager] Rust training returned no attribute deltas; using fallback to distribute growth."
						)
						_apply_simple_training(program, modifier, changes)

					var training_load_payload: Variant = response_type.get("training_load", {})
					if training_load_payload is Dictionary:
						training_load_snapshot = (training_load_payload as Dictionary).duplicate(true)
						needs_rest_warning = bool(training_load_snapshot.get("needs_rest", false))
						if needs_rest_warning:
							push_warning("[TrainingManager] ⚠️ 훈련 부하가 높습니다! 휴식을 고려하세요.")
					injury_risk_value = float(response_type.get("injury_risk", injury_risk_value))
				else:
					print("[TrainingManager] Unexpected training response type: %s" % payload_type)
					_apply_simple_training(program, modifier, changes)
			else:
				push_warning("[TrainingManager] Invalid response_type payload from Rust training engine")
				_apply_simple_training(program, modifier, changes)
		else:
			var error_code: String = str(result.get("error_code", ""))
			var failure_message: String = str(result.get("message", result.get("error_message", "Unknown error")))
			if error_code == "METHOD_NOT_FOUND":
				push_warning(
					"[TrainingManager] Rust training API missing (execute_training_json). Please rebuild GDExtension. Using simple fallback."
				)
			else:
				push_warning(
					"[TrainingManager] Rust training failed: %s, falling back to simple calculation" % failure_message
				)
			_apply_simple_training(program, modifier, changes)
	else:
		# Fallback to simple calculation if Rust engine unavailable
		push_warning("[TrainingManager] FootballRustEngine not ready, using simple fallback")
		_apply_simple_training(program, modifier, changes)

	if training_load_snapshot.is_empty() and _rest_caution_info.has("training_load"):
		var caution_load: Variant = _rest_caution_info.get("training_load", {})
		if caution_load is Dictionary:
			training_load_snapshot = (caution_load as Dictionary).duplicate(true)
	if not needs_rest_warning and training_load_snapshot.has("needs_rest"):
		needs_rest_warning = bool(training_load_snapshot.get("needs_rest", false))

	# Apply condition cost (training fatigue)
	ConditionSystem.apply_daily_change("training", -fatigue_cost, "훈련: %s" % program.name)

	print("[TrainingManager] Training completed: %s (Condition cost: -%.1f%%)" % [program.name, program.condition_cost])

	# Phase 9.2: Record training to history
	_record_training_history(
		training_id, program, changes, modifier, current_condition, training_load_snapshot, fatigue_cost
	)

	# Emit the calculated changes for the PlayerManager to handle.
	if not changes.is_empty():
		player_attributes_changed.emit(changes)

	# Prepare result
	var result = {
		"success": true,
		"changes": changes,
		"condition_cost": fatigue_cost,
		"message": "�Ʒ��� �Ϸ�Ǿ����ϴ�! %s" % program.name,
		"personal_training": is_personal,
		"training_name": program.name,
		"final_condition": ConditionSystem.get_condition_percentage(),
		"training_load": training_load_snapshot.duplicate(true),
		"injury_risk": injury_risk_value,
		"needs_rest_warning": needs_rest_warning,
		"rest_caution_active": _rest_caution_active,
		"rest_caution_info": _rest_caution_info.duplicate(true),
		"mode": context.get("mode", "personal"),
		"intensity": _frontend_intensity,
		"intensity_label": intensity_config.get("label", ""),
		"deck_bonus": _last_deck_bonus_info.duplicate(true) if _last_deck_bonus_info is Dictionary else {},
		"deck_snapshot": _last_deck_snapshot.duplicate()
	}

	# Emit completion signal
	var completed_payload = _build_training_event_payload("completed", training_id, program, context, result)
	training_completed.emit(completed_payload)

	# NEW: Emit general training finished signal for UI and other external systems
	training_finished.emit(result)

	if is_personal:
		personal_trainings_completed += 1

	# Phase 2: Clear special training state after use
	if CoachAffinitySystem and CoachAffinitySystem.is_special_training_active():
		CoachAffinitySystem.clear_special_training()

	# Phase 24: Log decision to DecisionTracker
	if DecisionTracker:
		# Build alternatives list (other available trainings)
		var alternatives = []
		for prog_id in TRAINING_PROGRAMS.keys():
			if prog_id != training_id and not prog_id.begins_with("special_"):
				alternatives.append(TRAINING_PROGRAMS[prog_id].name)

		# Calculate total CA gain from changes
		var ca_gain = 0
		for attr_value in changes.values():
			ca_gain += attr_value

		# Log the training decision
		DecisionTracker.log_decision(
			"training",
			program.name,
			alternatives,
			{
				"condition_before": current_condition,
				"ca_before": PlayerData.current_ca if PlayerData else 0,
				"week": GameManager.current_week if GameManager else 0,
				"intensity": _frontend_intensity,
				"mode": context.get("mode", "personal")
			},
			{
				"ca_gain": ca_gain,
				"condition_after": result.final_condition,
				"condition_loss": fatigue_cost,
				"injured": false,  # TODO: Hook injury system when implemented
				"changes": changes.duplicate()
			}
		)

	return result


func _apply_simple_training(program: Dictionary, modifier: float, changes: Dictionary) -> void:
	"""Fallback: Simple attribute increment without Rust growth calculations"""
	for attr in program.attributes:
		var increase = program.attributes[attr]
		var actual_increase = roundi(increase * modifier)

		# The PlayerManager will be responsible for applying this change.
		changes[attr] = actual_increase
		print(
			(
				"[TrainingManager] Simple growth: %s +%d (base: %d, modifier: %.2f)"
				% [attr, actual_increase, increase, modifier]
			)
		)


func can_execute_training(training_id: String, mode: String = "personal") -> Dictionary:
	"""
	Check if a training can be executed (without executing it)

	Args:
		training_id: ID of the training program

	Returns:
		Dictionary: {
			"can_execute": bool,
			"reason": String (if can't execute)
		}
	"""
	if not TRAINING_PROGRAMS.has(training_id):
		return {"can_execute": false, "reason": "훈련 프로그램을 찾을 수 없습니다"}

	if not ConditionSystem:
		return {"can_execute": false, "reason": "컨디션 시스템을 사용할 수 없습니다"}

	var current_condition = ConditionSystem.get_condition_percentage()
	if current_condition < MIN_CONDITION_TO_TRAIN:
		return {
			"can_execute": false,
			"reason": "컨디션이 너무 낮습니다 (%.1f%% < %.1f%%)" % [current_condition, MIN_CONDITION_TO_TRAIN]
		}
	var normalized_mode = mode if TRAINING_MODES.has(mode) else "personal"
	if normalized_mode != "team" and personal_trainings_completed >= MAX_PERSONAL_TRAININGS_PER_WEEK:
		return {"can_execute": false, "reason": "이번 주 개인 훈련 한도를 모두 사용했습니다"}

	return {"can_execute": true, "reason": ""}


func get_training_count_by_type(type: String) -> int:
	"""Get count of training programs of a specific type"""
	var count = 0
	for program_id in TRAINING_PROGRAMS:
		if TRAINING_PROGRAMS[program_id].type == type:
			count += 1
	return count


func get_trainings_by_type(type: String) -> Array:
	"""Get all training programs of a specific type"""
	var programs = []
	for program_id in TRAINING_PROGRAMS:
		var program = TRAINING_PROGRAMS[program_id]
		if program.type == type:
			programs.append(program)
	return programs


## Phase 9.2: Training History Functions


func _record_training_history(
	training_id: String,
	program: Dictionary,
	changes: Dictionary,
	modifier: float,
	initial_condition: float,
	training_load: Dictionary = {},
	condition_cost_override: float = -1.0
):
	"""Record a training session to history"""
	# Get current game date
	var year = 1
	var week = 1
	if DateManager:
		year = DateManager.current_year
		week = DateManager.current_week

	var record = {
		"training_id": training_id,
		"training_name": program.name,
		"training_type": program.type,
		"year": year,
		"week": week,
		"timestamp": Time.get_unix_time_from_system(),
		"attribute_changes": changes.duplicate(),
		"condition_before": initial_condition,
		"condition_after": ConditionSystem.get_condition_percentage() if ConditionSystem else 0.0,
		"condition_cost": condition_cost_override if condition_cost_override >= 0.0 else program.condition_cost,
		"effectiveness_modifier": modifier,
		"duration": program.duration,
		"training_load": training_load.duplicate(true) if training_load is Dictionary else {},
		"needs_rest_warning": bool(training_load.get("needs_rest", false)) if training_load is Dictionary else false
	}

	training_history.append(record)
	print("[TrainingManager] Recorded training history: %s (Week %d-%d)" % [program.name, year, week])


func get_training_history(limit: int = -1) -> Array:
	"""Get training history (most recent first)"""
	var history = training_history.duplicate()
	history.reverse()  # Most recent first

	if limit > 0 and history.size() > limit:
		return history.slice(0, limit)

	return history


func get_training_stats() -> Dictionary:
	"""Get training statistics"""
	var stats = {
		"total_sessions": training_history.size(),
		"sessions_by_type": {},
		"total_attribute_gains": {},
		"average_effectiveness": 0.0,
		"total_condition_cost": 0.0
	}

	if training_history.size() == 0:
		return stats

	var total_effectiveness = 0.0

	for record in training_history:
		# Count by type
		var type = record.get("training_type", "unknown")
		stats.sessions_by_type[type] = stats.sessions_by_type.get(type, 0) + 1

		# Sum attribute gains
		for attr in record.get("attribute_changes", {}):
			var gain = record.attribute_changes[attr]
			stats.total_attribute_gains[attr] = stats.total_attribute_gains.get(attr, 0) + gain

		# Sum effectiveness and condition cost
		total_effectiveness += record.get("effectiveness_modifier", 1.0)
		stats.total_condition_cost += record.get("condition_cost", 0.0)

	# Calculate average effectiveness
	stats.average_effectiveness = total_effectiveness / training_history.size()

	return stats


func clear_training_history():
	"""Clear all training history (for testing/reset)"""
	training_history.clear()
	print("[TrainingManager] Training history cleared")


func reset_weekly_state():
	personal_trainings_completed = 0
	print("[TrainingManager] Personal training counter reset")


func get_remaining_personal_trainings() -> int:
	return max(0, MAX_PERSONAL_TRAININGS_PER_WEEK - personal_trainings_completed)


func set_rest_caution(active: bool, info: Dictionary = {}):
	_rest_caution_active = active
	_rest_caution_info = info.duplicate(true) if info is Dictionary else {}
	if _rest_caution_active:
		print(
			(
				"[TrainingManager] Rest caution enabled (load_ratio=%.2f)"
				% float(_rest_caution_info.get("training_load", {}).get("load_ratio", 0.0))
			)
		)
	else:
		print("[TrainingManager] Rest caution cleared")


func save_to_dict() -> Dictionary:
	"""저장용 데이터 반환 (Phase 9.2)"""
	return {"training_history": training_history.duplicate(true)}


func load_from_dict(data: Dictionary):
	"""로드용 데이터 복원 (Phase 9.2)"""
	if data.has("training_history"):
		training_history = data["training_history"].duplicate(true)
		print("[TrainingManager] Loaded %d training records from save file" % training_history.size())
	else:
		training_history = []
		print("[TrainingManager] No training history in save file")


# ============================================
# Deck Bonus Integration
# ============================================


func _get_deck_training_bonus(training_type: String) -> float:
	"""
	Deck(SSOT: Rust/of_core::coach)에서 훈련 타입에 맞는 보너스 계산

	@param training_type: "technical", "physical", "mental", "tactical", "defensive"
	@return: 보너스 배수 (0.0 ~ 1.0, 예: 0.15 = +15%)
	"""

	if not FootballRustEngine or not FootballRustEngine.is_ready():
		_last_deck_bonus_info = {
			"success": false,
			"error": "Rust engine not ready",
			"training_type": training_type,
		}
		return 0.0

	if not FootballRustEngine.has_method("deck_get_active") or not FootballRustEngine.has_method("deck_calculate_training_bonus"):
		_last_deck_bonus_info = {
			"success": false,
			"error": "Deck bonus API not available",
			"training_type": training_type,
		}
		return 0.0

	var active := FootballRustEngine.deck_get_active()
	if not active.get("success", false):
		_last_deck_bonus_info = {
			"success": false,
			"error": active.get("error", "Failed to get active deck"),
			"training_type": training_type,
		}
		return 0.0

	var deck: Dictionary = active.get("deck", {})
	var bonus_result := FootballRustEngine.deck_calculate_training_bonus(deck, training_type)
	if not bonus_result.get("success", false):
		_last_deck_bonus_info = bonus_result.duplicate(true) if bonus_result is Dictionary else {}
		_last_deck_bonus_info["training_type"] = training_type
		return 0.0

	var total_multiplier: float = float(bonus_result.get("total_multiplier", 1.0))
	var total_bonus: float = max(0.0, total_multiplier - 1.0)

	var logs: Array = bonus_result.get("bonus_logs", [])
	var synergies: Array = bonus_result.get("active_synergies", [])

	_last_deck_bonus_info = {
		"success": true,
		"training_type": training_type,
		"deck_id": deck.get("deck_id", deck.get("id", "")),
		"deck_name": deck.get("deck_name", deck.get("name", "")),
		"deck_size": _last_deck_snapshot.size(),
		"total_multiplier": total_multiplier,
		"total_bonus": total_bonus,
		"bonus_logs": logs,
		"active_synergies": synergies,
	}

	if total_bonus > 0.0:
		var synergy_str := ", ".join(synergies) if synergies is Array and not synergies.is_empty() else "none"
		print(
			(
				"[TrainingManager] Deck bonus for %s training: x%.3f (+%.1f%%) synergies=%s"
				% [training_type, total_multiplier, total_bonus * 100.0, synergy_str]
			)
		)

	return total_bonus


# ============================================
# Week Growth Multiplier (Phase 1 Task 1.2)
# ============================================


func _get_week_growth_multiplier() -> float:
	"""
	주차별 CA 성장률 감소 곡선 적용

	Phase 1 (Week 1-50): 빠른 성장 (100%)
	Phase 2 (Week 51-100): 중속 성장 (62.5%)
	Phase 3 (Week 101-156): 느린 성장 (37.5%)

	@return: 성장률 배수 (1.0, 0.625, 또는 0.375)
	"""

	if not GameManager:
		return 1.0  # Fallback

	var week = GameManager.current_week

	if week <= 50:
		return 1.0  # 100% (초반 빠른 성장)
	elif week <= 100:
		return 0.625  # 62.5% (중반 감속)
	else:
		return 0.375  # 37.5% (후반 느림)


func perform_rest_activity() -> void:
	# 1. Check if activity can be done
	if GameManager and not GameManager.can_do_personal_activity():
		var result = {"success": false, "message": "이번 주 개인활동은 이미 완료했습니다"}
		rest_activity_completed.emit(result)
		return

	# 2. Mark activity as done
	if GameManager:
		GameManager.mark_personal_activity("rest")

	# 3. Simulate activity duration
	await get_tree().create_timer(1.5).timeout

	# 4. & 5. Perform logic and get results
	if not PlayerData:
		var result = {"success": false, "message": "PlayerData not found."}
		rest_activity_completed.emit(result)
		return

	var before_fatigue = PlayerData.fatigue
	var before_condition = PlayerData.condition

	PlayerData.rest_player()

	var after_fatigue = PlayerData.fatigue
	var after_condition = PlayerData.condition

	# 6. Create result dictionary and emit
	var result = {
		"success": true,
		"message": "😴 충분한 휴식을 통해 피로가 회복되었습니다!",
		"fatigue_before": before_fatigue,
		"fatigue_after": after_fatigue,
		"condition_before": before_condition,
		"condition_after": after_condition,
		"fatigue_recovered": before_fatigue - after_fatigue,
		"activity_type": "rest"  # To help UI identify the activity
	}
	rest_activity_completed.emit(result)


func perform_go_out_activity():
	# 1. Check if activity can be done
	if GameManager and not GameManager.can_do_personal_activity():
		var result = {"success": false, "message": "이번 주 개인활동은 이미 완료했습니다"}
		go_out_activity_completed.emit(result)
		return

	# 2. Mark activity as done
	if GameManager:
		GameManager.mark_personal_activity("go_out")

	# 3. & 4. & 5. Perform logic and get results
	if not PlayerData:
		var result = {"success": false, "message": "PlayerData not found."}
		go_out_activity_completed.emit(result)
		return

	var before_fatigue = PlayerData.fatigue
	var before_condition = PlayerData.condition

	# Resting is the base effect of going out
	PlayerData.rest_player()

	var after_fatigue = PlayerData.fatigue
	var after_condition = PlayerData.condition

	# 6. Create result dictionary and emit
	var result = {
		"success": true,
		"message": "🎮 기분 전환을 통해 리프레시되었습니다!",
		"fatigue_before": before_fatigue,
		"fatigue_after": after_fatigue,
		"condition_before": before_condition,
		"condition_after": after_condition,
		"fatigue_recovered": before_fatigue - after_fatigue,
		"activity_type": "go_out"
	}
	go_out_activity_completed.emit(result)


# ===============================================================================
# Phase 20: Helper Functions for Growth Profile and Personality
# ===============================================================================


func _get_player_specializations(position: String) -> Array:
	"""Infer specializations from player position (Phase 20)"""
	match position:
		"ST", "CF":
			return ["FINISHING", "PACE"]
		"CM":
			return ["PASSING", "VISION"]
		"CB":
			return ["TACKLING", "MARKING"]
		_:
			return []


func _get_technical_multiplier(player: Node) -> float:
	"""Calculate technical training multiplier based on player attributes (Phase 20)"""
	# Check if player is technical-focused (from attributes)
	var tech_avg = _get_category_average(player, "technical")
	if tech_avg > 70:
		return 1.2
	elif tech_avg > 60:
		return 1.1
	else:
		return 1.0


func _get_physical_multiplier(player: Node) -> float:
	"""Calculate physical training multiplier based on player attributes (Phase 20)"""
	var phys_avg = _get_category_average(player, "physical")
	if phys_avg > 70:
		return 1.2
	elif phys_avg > 60:
		return 1.1
	else:
		return 1.0


func _get_mental_multiplier(player: Node) -> float:
	"""Calculate mental training multiplier based on player attributes (Phase 20)"""
	var ment_avg = _get_category_average(player, "mental")
	if ment_avg > 70:
		return 1.2
	elif ment_avg > 60:
		return 1.1
	else:
		return 1.0


func _get_category_average(player: Node, category: String) -> float:
	"""Get average attribute value for a category (Phase 20)"""
	if not player.has_method("get_category_average"):
		return 50.0
	return player.get_category_average(category)
