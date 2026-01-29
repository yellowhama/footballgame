## DateManager.gd
## Autoload singleton for Academy Mode date-based turn system
## Manages 798 days (114 weeks, 3 years) of realistic football schedules
extends Node

# Preload to avoid autoload order issues with class_name
const _MatchFixture = preload("res://scripts/academy/MatchFixture.gd")
const _WeekSchedule = preload("res://scripts/academy/WeekSchedule.gd")
const _TrainingEventPayload = preload("res://scripts/utils/TrainingEventPayload.gd")
const WeeklyPlanResource = preload("res://scripts/model/WeeklyPlan.gd")
const TrainingPlanResource = preload("res://scripts/model/TrainingPlan.gd")

## Academy data structures available via global class_name declarations
## WeeklyPlan and TrainingPlan are also preloaded above to ensure availability

## ============================================================================
## SIGNALS - Time Progression
## ============================================================================

## Emitted at the start of each new day
## day_info: { day: int, day_of_week: int, week: int, year: int, period: Period }
signal day_started(day_info: Dictionary)

## Emitted when a new week starts (Monday)
## week_schedule: WeekSchedule object with this week's activities
signal week_started(week_number: int, week_schedule)

## Emitted when a week completes (Sunday night)
## week_stats: { training_count: int, matches: int, events: int, stamina_change: int, form_change: int }
## TODO: Future implementation - week completion statistics
# signal week_completed(week_stats: Dictionary)

## Emitted when a season ends (May Week 38)
## season_stats: { year: int, total_matches: int, wins: int, draws: int, losses: int, ca_growth: int }
signal season_ended(season_stats: Dictionary)

## Emitted when all 3 years complete
## final_stats: { total_turns: int, final_ca: int, achievements: Array }
signal academy_completed(final_stats: Dictionary)
signal rest_recommendation_changed(active: bool, info: Dictionary)
signal weekly_plan_updated(plan_data: Dictionary, changed_day: int)

## ============================================================================
## SIGNALS - Turn Lifecycle
## ============================================================================

## Emitted when a turn starts and requires player decision
## turn_type: "training" | "match_prep" | "match" | "event"
## turn_info: { day: int, available_actions: Array, context: Dictionary }
signal turn_decision_required(turn_type: String, turn_info: Dictionary)

## Emitted when a turn action is being processed
## turn_type: String
## action_data: Dictionary (varies by turn type)
signal turn_processing(turn_type: String, action_data: Dictionary)

## Emitted when a turn completes
## turn_results: { turn_type: String, success: bool, rewards: Dictionary, resource_changes: Dictionary }
signal turn_completed(turn_results: Dictionary)

## Emitted for automatic recovery turns (no decision needed)
## recovery_data: { stamina_before: int, stamina_after: int, forced: bool }
signal recovery_turn_processed(recovery_data: Dictionary)

## ============================================================================
## SIGNALS - Resource Changes
## ============================================================================

## Emitted when stamina changes
## old_value: int, new_value: int, reason: String
signal stamina_changed(old_value: int, new_value: int, reason: String)

## Emitted when form changes
## old_value: int, new_value: int, reason: String
signal form_changed(old_value: int, new_value: int, reason: String)

## Emitted when morale changes
## old_value: int, new_value: int, reason: String
signal morale_changed(old_value: int, new_value: int, reason: String)

## Emitted when stamina is critically low (< 30)
## current_stamina: int, forced_rest_required: bool
signal stamina_critical(current_stamina: int, forced_rest_required: bool)

## ============================================================================
## ENUMS
## ============================================================================

## Period types (coordinated with GameManager)
enum Period { SEASON, CAMP, VACATION }  ## 시즌 중 - 정규 훈련/경기  ## 합숙 - 집중 훈련  ## 방학 - 휴식 기간

## Turn state machine states
enum TurnState { IDLE, WAITING_DECISION, PROCESSING, COMPLETED }  ## No active turn  ## Waiting for player decision  ## Processing turn action  ## Turn completed, ready for next

## ============================================================================
## TIME TRACKING
## ============================================================================

## Current date tracking
var current_year: int = 1  ## 1-3 (U16/U17/U18)
var current_week: int = 1  ## 1-38 per year (season weeks only)
var current_day: int = 1  ## 1-798 total days
var current_day_of_week: int = 0  ## 0-6 (Monday=0, Sunday=6)

## Academic calendar
var season_start_week: int = 1  ## August Week 1
var season_end_week: int = 38  ## May Week 38
var total_days: int = 798  ## 3 years × 266 days/year

## MVP helper constants/state (Phase 7B)
const MVP_DAY_NAMES := ["monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday"]
const MVP_MAX_WEEKS_PER_YEAR: int = 12
var mvp_mode_enabled: bool = false
var mvp_completed: bool = false
var rest_recommendation_state: Dictionary = {"active": false}

## ============================================================================
## RESOURCE STATE (Player)
## ============================================================================

var stamina: int = 100  ## 0-100, from OpenFootball API
var form: int = 50  ## 0-100, Godot-managed
var morale: int = 50  ## 0-100, Godot-managed

## Match preparation quality (0.9-1.1 multiplier)
var last_prep_quality: float = 1.0  ## Applied to next match simulation

## ============================================================================
## TURN TRACKING
## ============================================================================

var active_turn_count: int = 0  ## Total active turns taken
var total_training_turns: int = 0
var total_match_prep_turns: int = 0
var total_match_turns: int = 0
var total_recovery_turns: int = 0
var total_event_turns: int = 0

## ============================================================================
## SCHEDULE DATA
## ============================================================================

var season_fixtures: Array = []  ## All fixtures for current season (MatchFixture)
var current_week_schedule = null  ## Current week's schedule (WeekSchedule)
var current_weekly_plan: Resource = WeeklyPlanResource.new()
var next_match_day: int = -1  ## Days until next match

## ============================================================================
## PERIOD SYSTEM
## ============================================================================

var current_period: Period = Period.SEASON

## Camp weeks by year
const CAMP_WEEKS = {
	1: [16, 17, 18, 19, 40, 41, 42, 43, 44, 45], 2: [16, 17, 18, 19, 20, 40, 41, 42, 43, 44, 45], 3: [40, 41, 42]  # U16: 10 weeks  # U17: 11 weeks  # U18: 3 weeks (pre-season only)
}

## Vacation weeks by year
const VACATION_WEEKS = {
	1: [1, 2, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 48, 49, 50, 51, 52],  # U16
	2: [1, 2, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 48, 49, 50, 51, 52],  # U17
	3: [1, 2, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 48, 49, 50, 51, 52]  # U18
}

## ============================================================================
## STATE MACHINE
## ============================================================================

var current_turn_state: TurnState = TurnState.IDLE
var current_turn_type: String = ""  ## "training", "match_prep", "match", "recovery", "event"
var current_turn_data: Dictionary = {}

## ============================================================================
## STAMINA THRESHOLDS
## ============================================================================

const STAMINA_THRESHOLDS = {"critical": 30, "warning": 50, "good": 70, "excellent": 85}  # Below: forced rest  # Below: injury risk +15%  # Above: optimal training  # Above: bonus training efficiency

## ============================================================================
## MATCH ID COUNTER
## ============================================================================

var _next_match_id: int = 1000

## ============================================================================
## INITIALIZATION
## ============================================================================


func _ready() -> void:
	print("[DateManager] Initializing...")

	# Generate initial season fixtures
	season_fixtures = generate_season_fixtures(current_year)

	# Generate initial week schedule
	current_week_schedule = generate_week_schedule(current_week, current_year)
	_connect_training_events()

	print("[DateManager] Initialized - Ready for Year %d, Week %d" % [current_year, current_week])
	print("[DateManager] Total fixtures for Year %d: %d" % [current_year, season_fixtures.size()])


func _connect_training_events() -> void:
	if EventBus:
		EventBus.subscribe("training_completed", _on_training_completed_event)


func _on_training_completed_event(event: Dictionary) -> void:
	record_training_activity(event)


func record_training_activity(event: Dictionary, day_of_week: int = -1) -> void:
	if event.is_empty():
		return
	if not current_weekly_plan:
		current_weekly_plan = WeeklyPlanResource.new()
	var normalized_event: Dictionary = (
		_TrainingEventPayload.normalize(event) if _TrainingEventPayload else event.duplicate(true)
	)
	var day_index: int = day_of_week
	if day_index < 0:
		day_index = current_day_of_week
	day_index = clampi(day_index, 0, 6)
	var entry: Dictionary = current_weekly_plan.get_day_entry(day_index)
	var raw_event: Dictionary = normalized_event.get("raw_event", event)
	var training_type: String = String(raw_event.get("type", entry.get("type", "training")))
	if training_type.is_empty():
		training_type = "training"
	entry["type"] = training_type
	entry["status"] = training_type
	var mode_id: String = String(normalized_event.get("mode_id", entry.get("mode", "personal")))
	entry["mode"] = mode_id
	entry["mode_label"] = normalized_event.get("mode_label", entry.get("mode_label", ""))
	entry["intensity"] = normalized_event.get("intensity_id", entry.get("intensity", "normal"))
	entry["intensity_label"] = normalized_event.get("intensity_label", entry.get("intensity_label", ""))
	entry["training_id"] = normalized_event.get("training_id", entry.get("training_id", ""))
	entry["training_name"] = normalized_event.get("training_name", entry.get("training_name", ""))
	entry["program_type"] = normalized_event.get("program_type", entry.get("program_type", ""))
	entry["description"] = normalized_event.get("description", entry.get("description", ""))
	entry["ui_note"] = normalized_event.get("ui_note", entry.get("ui_note", ""))
	var focus_value: Variant = raw_event.get("focus", entry.get("focus", ""))
	if typeof(focus_value) == TYPE_STRING and not String(focus_value).is_empty():
		entry["focus"] = String(focus_value)
	entry["deck_bonus"] = normalized_event.get("deck_bonus", entry.get("deck_bonus", {}))
	entry["deck_bonus_pct"] = normalized_event.get("deck_bonus_pct", entry.get("deck_bonus_pct", 0))
	entry["deck_snapshot"] = normalized_event.get("deck_snapshot", entry.get("deck_snapshot", []))
	entry["active_deck"] = normalized_event.get("active_deck", entry.get("active_deck", []))
	entry["changes"] = normalized_event.get("changes", entry.get("changes", {}))
	entry["training_load"] = normalized_event.get("training_load", entry.get("training_load", {}))
	entry["injury_risk"] = normalized_event.get("injury_risk", entry.get("injury_risk", -1.0))
	entry["needs_rest_warning"] = normalized_event.get("needs_rest_warning", entry.get("needs_rest_warning", false))
	entry["coach_bonus_log"] = normalized_event.get("coach_bonus_log", entry.get("coach_bonus_log", []))
	entry["timestamp"] = normalized_event.get("timestamp", entry.get("timestamp", Time.get_unix_time_from_system()))
	entry["result"] = normalized_event.get("result", entry.get("result", {}))
	entry["result_summary"] = _build_weekly_entry_summary(normalized_event)
	if mode_id == "special":
		entry["status"] = "special"
	current_weekly_plan.set_day_entry(day_index, entry)
	_emit_weekly_plan_update(day_index)


func _build_weekly_entry_summary(payload: Dictionary) -> Dictionary:
	if payload.is_empty():
		return {}
	var summary: Dictionary = {}
	var result: Dictionary = payload.get("result", {})
	var growth_variant: Variant = payload.get("changes", result.get("changes", {}))
	var growth: Dictionary = growth_variant if growth_variant is Dictionary else {}
	summary["changes"] = growth.duplicate(true)
	var load_variant: Variant = payload.get("training_load", result.get("training_load", {}))
	var load_data: Dictionary = load_variant if load_variant is Dictionary else {}
	summary["training_load"] = load_data.duplicate(true)
	summary["injury_risk"] = float(payload.get("injury_risk", result.get("injury_risk", -1.0)))
	summary["needs_rest_warning"] = bool(payload.get("needs_rest_warning", result.get("needs_rest_warning", false)))
	var coach_log_variant: Variant = payload.get("coach_bonus_log", result.get("coach_bonus_log", []))
	var coach_log: Array = coach_log_variant if coach_log_variant is Array else []
	summary["coach_bonus_log"] = coach_log.duplicate(true)
	if result.has("condition_cost"):
		summary["condition_cost"] = float(result.get("condition_cost", 0.0))
	if result.has("stamina_delta"):
		summary["stamina_delta"] = float(result.get("stamina_delta"))
	if result.has("success"):
		summary["success"] = bool(result.get("success"))
	if result.has("message"):
		var message := String(result.get("message", ""))
		if not message.is_empty():
			summary["message"] = message
	summary["mode"] = payload.get("mode_id", result.get("mode", "personal"))
	summary["intensity"] = payload.get("intensity_id", result.get("intensity", "normal"))
	summary["timestamp"] = int(payload.get("timestamp", result.get("timestamp", Time.get_unix_time_from_system())))
	return summary


func _emit_weekly_plan_update(changed_day: int = -1) -> void:
	if not is_inside_tree():
		return
	var payload: Dictionary = {}
	if current_weekly_plan and current_weekly_plan.has_method("to_dict"):
		payload = current_weekly_plan.to_dict()
	weekly_plan_updated.emit(payload, changed_day)


## ============================================================================
## TIME PROGRESSION METHODS
## ============================================================================


## Advance to next day and process turn
func advance_day() -> void:
	if current_turn_state != TurnState.IDLE and current_turn_state != TurnState.COMPLETED:
		push_error("[DateManager] Cannot advance day: turn not completed")
		return

	# Increment day
	current_day += 1
	current_day_of_week = (current_day_of_week + 1) % 7

	# Check week boundary (Monday = new week)
	if current_day_of_week == 0:
		current_week += 1
		_on_new_week()

	# Check year boundary
	if current_week > 38:
		_on_season_end()
		current_year += 1
		current_week = 1

		if current_year > 3:
			_on_academy_complete()
			return

	# Check academy completion
	if current_day > total_days:
		_on_academy_complete()
		return

	# Get current period
	current_period = _get_period(current_week, current_year)

	# Emit day started
	var day_info = {
		"day": current_day,
		"day_of_week": current_day_of_week,
		"week": current_week,
		"year": current_year,
		"period": current_period
	}
	day_started.emit(day_info)

	# Process daily turn
	_process_daily_turn()


## Process the turn for current day
func _process_daily_turn() -> void:
	# Determine turn type based on schedule
	var turn_type = _determine_turn_type(current_day_of_week, current_week_schedule)

	current_turn_type = turn_type
	current_turn_data = {}

	# Recovery turns are automatic
	if turn_type == "recovery":
		_process_automatic_recovery()
		return

	# Other turns require player decision
	current_turn_state = TurnState.WAITING_DECISION

	var turn_info = {
		"day": current_day,
		"day_of_week": current_day_of_week,
		"week": current_week,
		"year": current_year,
		"available_actions": _get_available_actions(turn_type),
		"context":
		{
			"stamina": stamina,
			"form": form,
			"morale": morale,
			"fixture": _get_fixture_for_day() if turn_type in ["match", "match_prep"] else null,
			"rest_recommendation":
			(
				rest_recommendation_state.duplicate(true)
				if rest_recommendation_state.get("active", false)
				else {"active": false}
			)
		}
	}

	turn_decision_required.emit(turn_type, turn_info)


## Execute turn action (called by UI after player decision)
func execute_turn_action(action_data: Dictionary) -> void:
	if current_turn_state != TurnState.WAITING_DECISION:
		push_error("[DateManager] Cannot execute turn: not waiting for decision")
		return

	current_turn_state = TurnState.PROCESSING
	current_turn_data = action_data

	turn_processing.emit(current_turn_type, action_data)

	# Delegate to appropriate manager
	match current_turn_type:
		"training":
			_execute_training_turn(action_data)
		"match_prep":
			_execute_match_prep_turn(action_data)
		"match":
			_execute_match_turn(action_data)
		"event":
			_execute_event_turn(action_data)


## Complete current turn (called by managers after processing)
func complete_turn(turn_type: String, results: Dictionary) -> void:
	if current_turn_state != TurnState.PROCESSING:
		push_error("[DateManager] Cannot complete turn: not processing")
		return

	current_turn_state = TurnState.COMPLETED

	# Update turn counters
	active_turn_count += 1
	match turn_type:
		"training":
			total_training_turns += 1
		"match_prep":
			total_match_prep_turns += 1
		"match":
			total_match_turns += 1
		"recovery":
			total_recovery_turns += 1
		"event":
			total_event_turns += 1

	# Apply resource changes
	if results.has("stamina_change"):
		update_stamina(results.stamina_change, results.get("reason", "turn_completed"))
	if results.has("form_change"):
		update_form(results.form_change, results.get("reason", "turn_completed"))
	if results.has("morale_change"):
		update_morale(results.morale_change, results.get("reason", "turn_completed"))

	# Emit completion
	var turn_results = {
		"turn_type": turn_type,
		"success": results.get("success", true),
		"rewards": results.get("rewards", {}),
		"resource_changes":
		{
			"stamina": results.get("stamina_change", 0),
			"form": results.get("form_change", 0),
			"morale": results.get("morale_change", 0)
		}
	}

	turn_completed.emit(turn_results)

	# Reset state
	current_turn_state = TurnState.IDLE


## ============================================================================
## RESOURCE MANAGEMENT METHODS
## ============================================================================


## Update stamina value
func update_stamina(delta: int, reason: String) -> void:
	var old_value = stamina
	stamina = clamp(stamina + delta, 0, 100)

	if stamina != old_value:
		stamina_changed.emit(old_value, stamina, reason)

	# Check critical threshold
	if stamina < 30:
		stamina_critical.emit(stamina, stamina < 20)


## Update form value
func update_form(delta: int, reason: String) -> void:
	var old_value = form
	form = clamp(form + delta, 0, 100)

	if form != old_value:
		form_changed.emit(old_value, form, reason)


## Update morale value
func update_morale(delta: int, reason: String) -> void:
	var old_value = morale
	morale = clamp(morale + delta, 0, 100)

	if morale != old_value:
		morale_changed.emit(old_value, morale, reason)


## Get current stamina status
func get_stamina_status() -> String:
	if stamina < STAMINA_THRESHOLDS.critical:
		return "critical"
	elif stamina < STAMINA_THRESHOLDS.warning:
		return "warning"
	elif stamina < STAMINA_THRESHOLDS.good:
		return "average"
	elif stamina < STAMINA_THRESHOLDS.excellent:
		return "good"
	else:
		return "excellent"


## Calculate event chance (dynamic)
func calculate_event_chance() -> float:
	var base_chance = 0.35

	# Relationship bonus (higher relationship = more events)
	var relationship_bonus = _get_average_relationship() * 0.001  # 0-10% bonus
	base_chance += relationship_bonus

	# Story progression bonus
	if _is_story_beat_active():
		base_chance += 0.05

	# Consecutive events penalty
	if _had_event_last_turn():
		base_chance -= 0.10

	# Camp period bonus
	if current_period == Period.CAMP:
		base_chance += 0.15

	return clamp(base_chance, 0.15, 0.60)


## ============================================================================
## FORM MANAGEMENT METHODS
## ============================================================================


## Calculate form change from training
func calculate_form_change_from_training(_training_type: String, intensity: String) -> int:
	# TODO: Use _training_type for training-specific form calculations
	var form_change = 0

	# Base change by intensity (from spec v3.0)
	match intensity:
		"intensive", "very_high":
			form_change = 8
		"normal", "moderate":
			form_change = 5
		"light", "very_light":
			form_change = 2
		_:
			form_change = 0

	# Stamina penalty (low stamina = reduced form gain)
	if stamina < STAMINA_THRESHOLDS.warning:
		form_change = int(form_change * 0.5)  # 50% reduction

	return form_change


## Calculate form change from match result
func calculate_form_change_from_match(match_result: Dictionary) -> int:
	var form_change = 0

	# Result-based form change (from spec v3.0)
	if match_result.get("is_win", false):
		form_change += 10
	elif match_result.get("is_draw", false):
		form_change += 3
	else:  # Loss
		form_change -= 5

	# Performance-based adjustment
	var player_rating = match_result.get("player_rating", 6.0)
	if player_rating >= 8.0:
		form_change += 5  # Excellent performance bonus
	elif player_rating >= 7.0:
		form_change += 2  # Good performance bonus
	elif player_rating < 6.0:
		form_change -= 3  # Poor performance penalty

	# Match importance multiplier
	var importance = match_result.get("importance", 1.0)
	if importance > 1.0:
		form_change = int(form_change * importance)

	return form_change


## Apply daily form decay (inactivity penalty)
func apply_daily_form_decay() -> void:
	# Form decays by 1 per day of inactivity (from spec v3.0)
	if current_turn_type not in ["training", "match"]:
		update_form(-1, "daily_inactivity")

	# Additional decay if form is very high (regression)
	if form > 70:
		update_form(-1, "form_regression")


## Apply enhanced stamina recovery
func apply_enhanced_stamina_recovery() -> void:
	var recovery_amount = 0

	# Daily sleep recovery (from spec v3.0: +5 per day)
	recovery_amount += 5

	# Weekend bonus (from spec v3.0: +10 on Sunday)
	if current_day_of_week == 6:  # Sunday
		recovery_amount += 10

	# Apply recovery
	if recovery_amount > 0:
		update_stamina(recovery_amount, "daily_recovery")


## ============================================================================
## AUTOMATIC RECOVERY PROCESSING
## ============================================================================


## Process automatic recovery turn (post-match or rest day)
func _process_automatic_recovery() -> void:
	if rest_recommendation_state.get("active", false):
		_clear_rest_recommendation("recovery_turn")
	var recovery_amount = 15  # Base recovery (from spec v3.0)

	# Weekend bonus
	if current_day_of_week == 6:  # Sunday
		recovery_amount += 10  # Total +25 on Sunday

	# Apply stamina recovery
	var stamina_before = stamina
	update_stamina(recovery_amount, "recovery_turn")

	# Form maintenance (slight decay if high)
	if form > 70:
		update_form(-2, "recovery_form_decay")

	# Injury risk reduction
	if stamina >= STAMINA_THRESHOLDS.good:
		# TODO: Reduce injury_risk variable when implemented
		pass

	# Emit recovery signal
	recovery_turn_processed.emit(
		{
			"stamina_before": stamina_before,
			"stamina_after": stamina,
			"recovery_amount": recovery_amount,
			"forced": stamina < STAMINA_THRESHOLDS.critical
		}
	)

	# Auto-advance to next day
	await get_tree().create_timer(0.1).timeout
	advance_day()


## ============================================================================
## MATCH PREPARATION METHODS
## ============================================================================


## Calculate preparation quality from match prep actions
func calculate_match_prep_quality(prep_actions: Dictionary) -> float:
	var quality = 1.0

	# Formation set correctly (+5%)
	if prep_actions.has("formation") and prep_actions.formation != "":
		quality += 0.05

	# Instructions aligned with team strength (+3%)
	if prep_actions.has("instructions"):
		quality += 0.03

	# Team talk boosts morale (+2% to quality)
	if prep_actions.has("team_talk"):
		update_morale(5, "team_talk")
		quality += 0.02

	# Item usage (+5%)
	if prep_actions.has("item_used"):
		quality += 0.05

	# Opponent analysis (+5%)
	if prep_actions.has("opponent_analyzed"):
		quality += 0.05

	# Stamina affects preparation quality
	if stamina < STAMINA_THRESHOLDS.warning:
		quality -= 0.05  # Fatigue affects preparation

	return clamp(quality, 0.9, 1.1)


## Execute match prep turn
func _execute_match_prep_turn(action_data: Dictionary) -> void:
	# Calculate preparation quality
	last_prep_quality = calculate_match_prep_quality(action_data)

	# Store prep data for match turn
	current_turn_data.prep_quality = last_prep_quality
	current_turn_data.formation = action_data.get("formation", "4-4-2")
	current_turn_data.instructions = action_data.get("instructions", "balanced")

	# Apply any item effects
	if action_data.has("item_used"):
		_apply_item_effect(action_data.item_used)

	# Complete prep turn
	complete_turn(
		"match_prep",
		{"success": true, "prep_quality": last_prep_quality, "morale_change": 5 if action_data.has("team_talk") else 0}
	)


## Apply item effects during match prep
func _apply_item_effect(item_id: String) -> void:
	# TODO: Implement item system
	match item_id:
		"energy_drink":
			update_stamina(10, "energy_drink")
		"focus_supplement":
			update_form(5, "focus_supplement")
		"team_meal":
			update_morale(10, "team_meal")


## ============================================================================
## TURN EXECUTION STUBS (to be implemented with manager integration)
## ============================================================================


func _execute_training_turn(action_data: Dictionary) -> void:
	"""Execute training turn by delegating to TrainingManager"""
	if not TrainingManager:
		push_error("[DateManager] TrainingManager not available")
		complete_turn("training", {"success": false, "error": "TrainingManager not found"})
		return

	var training_program = action_data.get("program", "")
	if training_program == "":
		push_error("[DateManager] No training program specified")
		complete_turn("training", {"success": false, "error": "No program specified"})
		return

	print("[DateManager] Executing training turn: %s" % training_program)

	# Execute training through TrainingManager
	var result = TrainingManager.execute_training(training_program)

	# Calculate form change from training
	var intensity = result.get("intensity", "normal")
	var form_change = calculate_form_change_from_training(training_program, intensity)

	if result.get("needs_rest_warning", false):
		_activate_rest_recommendation(training_program, result)

	# Complete turn with results
	complete_turn(
		"training",
		{
			"success": result.get("success", false),
			"program": training_program,
			"stamina_change": result.get("stamina_cost", 0) * -1,  # Negative cost
			"form_change": form_change,
			"morale_change": 0,
			"rewards": result.get("rewards", {})
		}
	)


func _execute_match_turn(action_data: Dictionary) -> void:
	"""Execute match turn by delegating to MatchManager"""
	if not MatchManager:
		push_error("[DateManager] MatchManager not available")
		complete_turn("match", {"success": false, "error": "MatchManager not found"})
		return

	# Get fixture for this match day
	var fixture = _get_fixture_for_day()
	if not fixture:
		push_error("[DateManager] No fixture found for match day")
		complete_turn("match", {"success": false, "error": "No fixture found"})
		return

	print("[DateManager] Executing match turn vs %s" % fixture.opponent_name)

	# Set tactic if provided
	var tactic = action_data.get("tactic", "균형")

	# Build opponent data from fixture
	var opponent = {"name": fixture.opponent_name, "overall_rating": fixture.opponent_strength}

	# Start match through MatchManager
	MatchManager.start_match(opponent)

	# Apply tactic
	if tactic != "균형":
		MatchManager.set_tactic(tactic)

	# Connect to match_ended signal to get results
	if not MatchManager.match_ended.is_connected(_on_match_completed):
		MatchManager.match_ended.connect(_on_match_completed)


func _on_match_completed(result: Dictionary):
	"""Handle match completion callback"""
	# Calculate form change from match result
	var match_result_data = {
		"is_win": result.get("result", "") == "승리",
		"is_draw": result.get("result", "") == "무승부",
		"player_rating": 7.0,  # TODO: Get actual player rating from match simulation
		"importance": 1.0
	}

	var form_change = calculate_form_change_from_match(match_result_data)

	# Complete turn with match results
	complete_turn(
		"match",
		{
			"success": true,
			"result": result.get("result", ""),
			"final_score": result.get("final_score", [0, 0]),
			"stamina_change": -25,  # Matches cost 25 stamina
			"form_change": form_change,
			"morale_change": 10 if match_result_data.is_win else (0 if match_result_data.is_draw else -5),
			"rewards": result
		}
	)


func _execute_event_turn(action_data: Dictionary) -> void:
	"""Execute event turn (basic recovery/rest implementation)"""
	# For now, implement as enhanced recovery turn
	# TODO: Integrate with EventManager when story events system is ready

	var event_choice = action_data.get("choice", "rest")

	print("[DateManager] Executing event turn: %s" % event_choice)

	# Basic event effects
	var stamina_change = 0
	var morale_change = 0
	var form_change = 0

	match event_choice:
		"rest":
			# Full rest day
			stamina_change = 20
			form_change = -2  # Slight form decay
			_clear_rest_recommendation("event_rest")
		"light_training":
			# Light recovery training
			stamina_change = 10
			form_change = 3
		"team_activity":
			# Team building activity
			stamina_change = 5
			morale_change = 10
			form_change = 0

	# Complete turn with event results
	complete_turn(
		"event",
		{
			"success": true,
			"event_choice": event_choice,
			"stamina_change": stamina_change,
			"form_change": form_change,
			"morale_change": morale_change,
			"rewards": {}
		}
	)


func _activate_rest_recommendation(training_id: String, result: Dictionary) -> void:
	var load_info: Dictionary = result.get("training_load", {})
	rest_recommendation_state = {
		"active": true,
		"issued_week": current_week,
		"issued_day": current_day,
		"training_id": training_id,
		"injury_risk": result.get("injury_risk", -1.0),
		"training_load": load_info.duplicate(true) if load_info is Dictionary else {},
		"message": "코치가 휴식을 권장합니다. 과도한 훈련 부하가 감지되었습니다."
	}
	rest_recommendation_changed.emit(true, rest_recommendation_state.duplicate(true))

	if TrainingManager and TrainingManager.has_method("set_rest_caution"):
		TrainingManager.set_rest_caution(true, rest_recommendation_state)

	_notify_coach_rest_warning(rest_recommendation_state)


func _clear_rest_recommendation(reason: String = "manual") -> void:
	if not rest_recommendation_state.get("active", false):
		return
	var cleared = rest_recommendation_state.duplicate(true)
	cleared["active"] = false
	cleared["cleared_reason"] = reason
	rest_recommendation_state = {"active": false}
	rest_recommendation_changed.emit(false, cleared)

	if TrainingManager and TrainingManager.has_method("set_rest_caution"):
		TrainingManager.set_rest_caution(false)


func _notify_coach_rest_warning(info: Dictionary) -> void:
	var coach_system = get_node_or_null("/root/CoachSystem")
	if coach_system and coach_system.has_method("issue_rest_warning"):
		coach_system.issue_rest_warning(info)


## ============================================================================
## HELPER METHODS (Placeholders for now)
## ============================================================================


func _get_average_relationship() -> float:
	# TODO: Integrate with relationship system
	return 50.0


func _is_story_beat_active() -> bool:
	# TODO: Integrate with story system
	return false


func _had_event_last_turn() -> bool:
	# TODO: Track last turn type
	return false


func _get_available_actions(_turn_type: String) -> Array:
	# TODO: Return available actions based on _turn_type
	return []


func _get_fixture_for_day():  # Returns MatchFixture or null
	if current_week_schedule == null:
		return null

	return current_week_schedule.call("get_fixture_for_day", current_day_of_week)


func get_current_weekly_plan():  # Returns WeeklyPlan
	return current_weekly_plan


func apply_weekly_plan_dict(plan_dict: Dictionary, emit_update: bool = true) -> void:
	if plan_dict.is_empty():
		return
	current_weekly_plan = WeeklyPlanResource.from_dict(plan_dict)
	if emit_update:
		_emit_weekly_plan_update()


## ============================================================================
## PERIOD HELPER METHODS
## ============================================================================


## Get period for given week and year
func _get_period(week: int, year: int) -> Period:
	# Check vacation weeks
	if week in VACATION_WEEKS.get(year, []):
		return Period.VACATION

	# Check camp weeks
	if week in CAMP_WEEKS.get(year, []):
		return Period.CAMP

	# Default to season
	return Period.SEASON


## ============================================================================
## WEEK/SEASON EVENT HANDLERS (Stubs for now)
## ============================================================================


func _on_new_week() -> void:
	_clear_rest_recommendation("new_week")
	# Generate new week schedule
	current_week_schedule = generate_week_schedule(current_week, current_year)

	# Emit week started signal
	week_started.emit(current_week, current_week_schedule)

	# Phase 24: Track weekly CA progression
	if CareerStatisticsManager and PlayerData:
		var current_ca = PlayerData.current_ca if PlayerData.has("current_ca") else 0
		CareerStatisticsManager.track_weekly_ca(current_week, current_ca)

	print("[DateManager] Week %d started (Year %d)" % [current_week, current_year])


func _on_season_end() -> void:
	print("[DateManager] Season %d ending..." % current_year)

	# ✅ Phase 23: Get division summary from DivisionManager
	var division_summary = {}
	if DivisionManager:
		division_summary = DivisionManager.get_season_summary()

	var season_stats = {
		"year": current_year,
		"total_matches": division_summary.get("played", 0),
		"wins": division_summary.get("won", 0),
		"draws": division_summary.get("drawn", 0),
		"losses": division_summary.get("lost", 0),
		"division": division_summary.get("division", 3),
		"division_name": division_summary.get("division_name", ""),
		"final_position": division_summary.get("position", -1),
		"points": division_summary.get("points", 0),
		"goals_for": division_summary.get("gf", 0),
		"goals_against": division_summary.get("ga", 0),
		"ca_growth": 0  # TODO: Get from PlayerManager
	}

	# Emit season ended signal (for UI)
	season_ended.emit(season_stats)

	# ✅ Phase 23: Process promotion/relegation
	if DivisionManager:
		var outcome = DivisionManager.check_promotion_relegation()

		match outcome.outcome:
			"promoted":
				DivisionManager.player_promoted.emit(outcome.old_division, outcome.new_division)
				print(
					(
						"[DateManager] 🎉 PROMOTED to Division %d! (Position: %d/6, Points: %d)"
						% [outcome.new_division, outcome.final_position, outcome.points]
					)
				)

			"relegated":
				DivisionManager.player_relegated.emit(outcome.old_division, outcome.new_division)
				print(
					(
						"[DateManager] 😞 Relegated to Division %d (Position: %d/6, Points: %d)"
						% [outcome.new_division, outcome.final_position, outcome.points]
					)
				)

			"stayed":
				DivisionManager.player_stayed.emit(outcome.old_division, outcome.final_position)
				print(
					(
						"[DateManager] Stayed in Division %d, Position %d/6 (Points: %d)"
						% [outcome.old_division, outcome.final_position, outcome.points]
					)
				)

		# Start new season in new/same division
		if current_year < 3:
			DivisionManager.start_new_season(current_year + 1)

		# Phase 24: Record season division for career statistics
		if CareerStatisticsManager:
			var promoted = outcome.outcome == "promoted"
			var relegated = outcome.outcome == "relegated"
			CareerStatisticsManager.record_season_division(
				current_year,
				outcome.get("old_division", season_stats.division),
				outcome.get("final_position", season_stats.final_position),
				promoted,
				relegated
			)

	print("[DateManager] Season %d ended" % current_year)

	# Generate fixtures for next season
	if current_year < 3:
		season_fixtures = generate_season_fixtures(current_year + 1)


func _on_academy_complete() -> void:
	# Compile final stats
	var final_stats = {"total_turns": active_turn_count, "final_ca": 0, "achievements": []}  # TODO: Get from player data

	# Emit academy completed signal
	academy_completed.emit(final_stats)

	print("[DateManager] Academy Mode completed!")


## ============================================================================
## SCHEDULE GENERATION METHODS
## ============================================================================


## Generate season fixtures for given year
func generate_season_fixtures(year: int) -> Array:
	var fixtures: Array = []  # Array of MatchFixture

	# Match distribution by year (from MatchFixture_Design.md)
	var match_distribution = _get_match_distribution(year)
	var total_matches = match_distribution.values().reduce(func(acc, val): return acc + val, 0)

	# Midweek match ratio
	var midweek_ratios = {1: 0.2, 2: 0.3, 3: 0.35}
	var midweek_count = int(total_matches * midweek_ratios[year])
	var saturday_count = total_matches - midweek_count

	# Season weeks (August-May = weeks 1-38)
	var available_weeks = range(1, 39)

	# Remove CAMP weeks (no matches during training camps)
	var camp_weeks = CAMP_WEEKS[year]
	for camp_week in camp_weeks:
		available_weeks.erase(camp_week)

	# Remove VACATION weeks
	var vacation_weeks = VACATION_WEEKS[year]
	for vacation_week in vacation_weeks:
		available_weeks.erase(vacation_week)

	# Seed for reproducible fixtures
	var fixture_seed = 10000 + year * 100
	seed(fixture_seed)
	available_weeks.shuffle()

	var fixture_index = 0
	var week_pool = available_weeks.duplicate()
	var saturday_allocated = 0

	# 1. Generate League matches (most common)
	for i in range(match_distribution.league):
		if week_pool.is_empty():
			break

		var week = week_pool.pop_front()
		var day_of_week = 5 if saturday_allocated < saturday_count else 2  # Saturday or Wednesday
		if day_of_week == 5:
			saturday_allocated += 1

		var fixture = _MatchFixture.new(_MatchFixture.MatchType.LEAGUE, year, week, day_of_week)
		fixture.fixture_index = fixture_index
		fixture.match_id = _generate_match_id()
		fixture.absolute_day = _calculate_absolute_day(year, week, day_of_week)
		fixture.is_home_match = (fixture_index % 2 == 0)
		fixture.opponent_name = _generate_opponent_name(year)
		fixture.opponent_strength = randi_range(40, 60)

		fixtures.append(fixture)
		fixture_index += 1

	# 2. Generate Cup matches (higher importance weeks)
	for i in range(match_distribution.cup):
		if week_pool.is_empty():
			break

		var week = _select_important_week(week_pool)
		var day_of_week = 5  # Saturday for cup matches

		var fixture = _MatchFixture.new(_MatchFixture.MatchType.CUP, year, week, day_of_week)
		fixture.fixture_index = fixture_index
		fixture.match_id = _generate_match_id()
		fixture.absolute_day = _calculate_absolute_day(year, week, day_of_week)
		fixture.is_home_match = (i % 2 == 0)
		fixture.opponent_name = _generate_opponent_name(year, "Cup")
		fixture.opponent_strength = randi_range(45, 65)  # Harder opponents

		fixtures.append(fixture)
		fixture_index += 1
		week_pool.erase(week)

	# 3. Generate Friendly matches (low pressure weeks)
	for i in range(match_distribution.friendly):
		if week_pool.is_empty():
			break

		var week = week_pool.pop_front()
		var day_of_week = 5

		var fixture = _MatchFixture.new(_MatchFixture.MatchType.FRIENDLY, year, week, day_of_week)
		fixture.fixture_index = fixture_index
		fixture.match_id = _generate_match_id()
		fixture.absolute_day = _calculate_absolute_day(year, week, day_of_week)
		fixture.is_home_match = true  # Usually home friendlies
		fixture.opponent_name = _generate_opponent_name(year, "Friendly")
		fixture.opponent_strength = randi_range(35, 55)  # Easier opponents

		fixtures.append(fixture)
		fixture_index += 1

	# 4. Generate UEFA Youth League matches (if applicable)
	if match_distribution.uefa_youth_league > 0:
		for i in range(match_distribution.uefa_youth_league):
			if week_pool.is_empty():
				break

			var week = _select_uefa_week(week_pool)
			var day_of_week = 2  # Wednesday for UEFA

			var fixture = _MatchFixture.new(_MatchFixture.MatchType.UEFA_YOUTH_LEAGUE, year, week, day_of_week)
			fixture.fixture_index = fixture_index
			fixture.match_id = _generate_match_id()
			fixture.absolute_day = _calculate_absolute_day(year, week, day_of_week)
			fixture.is_home_match = (i % 2 == 0)
			fixture.opponent_name = _generate_opponent_name(year, "UEFA")
			fixture.opponent_strength = randi_range(55, 75)  # Hardest opponents

			fixtures.append(fixture)
			fixture_index += 1
			week_pool.erase(week)

	# Sort by absolute day
	fixtures.sort_custom(
		func(a, b):
			if a.scheduled_week != b.scheduled_week:
				return a.scheduled_week < b.scheduled_week
			return a.scheduled_day_of_week < b.scheduled_day_of_week
	)

	print("[DateManager] Generated %d fixtures for Year %d" % [fixtures.size(), year])
	return fixtures


## Generate week schedule for given week and year
func generate_week_schedule(week_number: int, year: int):  # Returns WeekSchedule
	var schedule = _WeekSchedule.new(week_number, year)

	# Check period
	var period = _get_period(week_number, year)
	schedule.period = period

	if period == Period.VACATION:
		# Vacation week: all rest days - already handled by _WeekSchedule._init()
		return schedule

	# Check if this week has a match
	var fixtures_this_week = _get_fixtures_for_week(week_number, year)

	if not fixtures_this_week.is_empty():
		for fixture in fixtures_this_week:
			schedule.call("add_fixture", fixture)

	_annotate_week_schedule(schedule, fixtures_this_week)
	current_weekly_plan = _build_weekly_plan_from_schedule(schedule)
	_emit_weekly_plan_update()

	return schedule


func _annotate_week_schedule(schedule, fixtures: Array) -> void:
	var primary_fixture: _MatchFixture = fixtures[0] if fixtures.size() > 0 else null
	for day in range(7):
		var plan: Dictionary = schedule.call("get_day_plan", day)
		var day_type := String(plan.get("type", "rest"))
		match day_type:
			"match":
				var fixture = schedule.call("get_fixture_for_day", day)
				if fixture:
					plan["opponent"] = fixture.opponent_name
					plan["match_type"] = fixture.get_match_type_name()
					plan["importance"] = fixture.importance
			"prep":
				if primary_fixture:
					plan["focus"] = "전술/세트피스"
					plan["opponent"] = primary_fixture.opponent_name
					plan["days_until_match"] = max(primary_fixture.scheduled_day_of_week - day, 0)
			"training":
				var training_plan = _build_training_plan_for_day(day, primary_fixture)
				for key in training_plan.keys():
					plan[key] = training_plan[key]
				var intensity_id: String = String(plan.get("intensity", "normal"))
				plan["intensity_label"] = _describe_training_intensity_label(intensity_id)
			"recovery":
				plan["focus"] = "회복"
			_:
				pass
		schedule.call("set_day_plan", day, plan)


func _describe_training_intensity_label(intensity_id: String) -> String:
	var key: String = ""
	match intensity_id:
		"light":
			key = "UI_TRAINING_INTENSITY_LIGHT"
		"normal":
			key = "UI_TRAINING_INTENSITY_NORMAL"
		"intense":
			key = "UI_TRAINING_INTENSITY_INTENSE"
		_:
			key = ""
	if key.is_empty():
		return intensity_id.capitalize()
	return tr(key)


func _build_training_plan_for_day(day: int, fixture) -> Dictionary:
	var plan: Dictionary = {"mode": "personal", "focus": "개인 성장", "intensity": "normal"}
	if fixture == null:
		return plan
	var days_until: int = fixture.scheduled_day_of_week - day
	if days_until <= 1:
		plan["mode"] = "team"
		plan["focus"] = "세트피스"
		plan["intensity"] = "light"
	elif days_until <= 3:
		plan["mode"] = "team"
		plan["focus"] = "전술 조직"
		plan["intensity"] = "normal"
	else:
		plan["mode"] = "personal"
		plan["focus"] = "피지컬"
		plan["intensity"] = "intense"
	plan["opponent"] = fixture.opponent_name
	return plan


func _build_weekly_plan_from_schedule(schedule):  # Returns WeeklyPlan
	var weekly_plan = WeeklyPlanResource.new()
	if schedule == null:
		return weekly_plan

	weekly_plan.team_plan = {"week_type": schedule.week_type, "period": schedule.period}

	for day in range(7):
		var entry: Dictionary = schedule.call("get_day_plan", day)
		weekly_plan.set_day_entry(day, entry)
		if (
			weekly_plan.personal_plan == null
			and String(entry.get("mode", "")) == "personal"
			and String(entry.get("type", "")) == "training"
		):
			var training_plan = TrainingPlanResource.new()
			training_plan.kind = String(entry.get("focus", training_plan.kind))
			var intensity_str := String(entry.get("intensity", "normal"))
			match intensity_str:
				"light":
					training_plan.intensity = 0.7
				"intense":
					training_plan.intensity = 1.3
				_:
					training_plan.intensity = 1.0
			weekly_plan.set_training(training_plan)

	if weekly_plan.personal_plan == null:
		weekly_plan.set_rest()

	return weekly_plan


## Determine turn type for current day
func _determine_turn_type(day_of_week: int, week_schedule) -> String:  # week_schedule: WeekSchedule
	if week_schedule == null:
		return "recovery"

	# Recovery Turn (automatic, no decision) - using .call() to bypass static parser
	if week_schedule.call("is_recovery_day", day_of_week):
		return "recovery"

	# Match Day
	if week_schedule.call("has_match", day_of_week):
		return "match"

	# Match Prep Day (day before match)
	if week_schedule.call("is_match_prep_day", day_of_week):
		return "match_prep"

	# Training Day (default weekday activity)
	if week_schedule.call("is_training_day", day_of_week):
		# 35-40% chance for Event Turn
		if randf() < calculate_event_chance():
			return "event"
		else:
			return "training"

	# Vacation/Rest day
	return "recovery"


## ============================================================================
## SCHEDULE GENERATION HELPER METHODS
## ============================================================================


## Get match distribution by year
func _get_match_distribution(year: int) -> Dictionary:
	match year:
		1:  # U16
			return {"league": 18, "cup": 3, "friendly": 3, "uefa_youth_league": 0}
		2:  # U17
			return {"league": 20, "cup": 4, "friendly": 2, "uefa_youth_league": 2}
		3:  # U18
			return {"league": 22, "cup": 5, "friendly": 1, "uefa_youth_league": 4}
		_:
			return {"league": 18, "cup": 3, "friendly": 3, "uefa_youth_league": 0}


## Generate match ID
func _generate_match_id() -> int:
	_next_match_id += 1
	return _next_match_id


## Calculate absolute day
func _calculate_absolute_day(year: int, week: int, day_of_week: int) -> int:
	var year_offset = (year - 1) * 266  # Days per year
	var week_offset = (week - 1) * 7
	return year_offset + week_offset + day_of_week + 1


## Generate opponent name
func _generate_opponent_name(year: int, match_type_suffix: String = "") -> String:
	var age_groups = {1: "U16", 2: "U17", 3: "U18"}
	var teams = [
		"Arsenal",
		"Chelsea",
		"Liverpool",
		"Manchester City",
		"Manchester United",
		"Tottenham",
		"West Ham",
		"Everton",
		"Aston Villa",
		"Newcastle",
		"Southampton",
		"Fulham",
		"Brighton",
		"Brentford",
		"Crystal Palace"
	]

	var team = teams[randi() % teams.size()]
	var age = age_groups[year]

	if match_type_suffix != "":
		return "%s %s (%s)" % [team, age, match_type_suffix]
	else:
		return "%s %s" % [team, age]


## Select important week for Cup matches
func _select_important_week(week_pool: Array) -> int:
	# Prefer weeks after week 10 (season established)
	var important_weeks = week_pool.filter(func(w): return w >= 10)
	if important_weeks.is_empty():
		return week_pool[0]
	return important_weeks[randi() % important_weeks.size()]


## Select UEFA week
func _select_uefa_week(week_pool: Array) -> int:
	# UEFA matches cluster in specific periods
	var uefa_periods = week_pool.filter(func(w): return (w >= 8 and w <= 12) or (w >= 25 and w <= 30))
	if uefa_periods.is_empty():
		return week_pool[0]
	return uefa_periods[randi() % uefa_periods.size()]


## Get fixtures for specific week
func _get_fixtures_for_week(week: int, year: int) -> Array:
	var result: Array = []  # Array of MatchFixture
	for fixture in season_fixtures:
		if fixture.scheduled_week == week and fixture.scheduled_year == year:
			result.append(fixture)
	return result


## ============================================================================
## SAVE/LOAD SYSTEM
## ============================================================================


func save_state() -> Dictionary:
	return {
		"version": 1,
		"current_year": current_year,
		"current_week": current_week,
		"current_day": current_day,
		"current_day_of_week": current_day_of_week,
		"stamina": stamina,
		"form": form,
		"morale": morale,
		"active_turn_count": active_turn_count,
		"turn_counters":
		{
			"training": total_training_turns,
			"match_prep": total_match_prep_turns,
			"match": total_match_turns,
			"recovery": total_recovery_turns,
			"event": total_event_turns
		},
		"season_fixtures": season_fixtures.map(func(f): return f.to_dict()),
		"current_turn_state": current_turn_state,
		"current_turn_type": current_turn_type
	}


func load_state(data: Dictionary) -> void:
	if data.version != 1:
		push_error("[DateManager] Incompatible save version")
		return

	current_year = data.current_year
	current_week = data.current_week
	current_day = data.current_day
	current_day_of_week = data.current_day_of_week
	stamina = data.stamina
	form = data.form
	morale = data.morale
	active_turn_count = data.active_turn_count

	total_training_turns = data.turn_counters.training
	total_match_prep_turns = data.turn_counters.match_prep
	total_match_turns = data.turn_counters.match
	total_recovery_turns = data.turn_counters.recovery
	total_event_turns = data.turn_counters.event

	# Restore fixtures
	season_fixtures.clear()
	for fixture_data in data.season_fixtures:
		var fixture = _MatchFixture.from_dict(fixture_data)
		season_fixtures.append(fixture)

	current_turn_state = data.current_turn_state
	current_turn_type = data.current_turn_type

	# Regenerate current week schedule
	current_week_schedule = generate_week_schedule(current_week, current_year)

	print("[DateManager] State loaded: Day %d, Week %d, Year %d" % [current_day, current_week, current_year])


## ============================================================================
## TESTING SUPPORT METHODS
## ============================================================================


## Convert state to dictionary (alias for save_state for test compatibility)
func to_dict() -> Dictionary:
	return save_state()


## Load state from dictionary (alias for load_state for test compatibility)
func from_dict(data: Dictionary) -> void:
	load_state(data)


## Reset to initial state for testing
func reset_for_testing() -> void:
	current_year = 1
	current_week = 1
	current_day = 1
	current_day_of_week = 0
	stamina = 100
	form = 50
	morale = 50
	active_turn_count = 0
	total_training_turns = 0
	total_match_prep_turns = 0
	total_match_turns = 0
	total_recovery_turns = 0
	total_event_turns = 0
	current_turn_state = TurnState.IDLE
	current_turn_type = ""
	current_turn_data = {}
	last_prep_quality = 1.0

	# Regenerate fixtures and schedule
	season_fixtures = generate_season_fixtures(current_year)
	current_week_schedule = generate_week_schedule(current_week, current_year)

	print("[DateManager] Reset to initial state for testing")


func save_to_dict() -> Dictionary:
	return {
		"current_year": current_year,
		"current_week": current_week,
		"current_day": current_day,
		"current_day_of_week": current_day_of_week,
		"mvp_mode_enabled": mvp_mode_enabled,
		"mvp_completed": mvp_completed,
		"weekly_plan":
		current_weekly_plan.to_dict() if current_weekly_plan and current_weekly_plan.has_method("to_dict") else {}
	}


func load_from_dict(data: Dictionary) -> void:
	if data.is_empty():
		return

	current_year = int(data.get("current_year", current_year))
	current_week = int(data.get("current_week", current_week))
	current_day = int(data.get("current_day", current_day))
	current_day_of_week = clampi(int(data.get("current_day_of_week", current_day_of_week)), 0, 6)
	mvp_mode_enabled = bool(data.get("mvp_mode_enabled", mvp_mode_enabled))
	mvp_completed = bool(data.get("mvp_completed", mvp_completed))

	current_week_schedule = generate_week_schedule(current_week, current_year)
	if data.has("weekly_plan"):
		apply_weekly_plan_dict(data["weekly_plan"], true)
	else:
		current_weekly_plan = _build_weekly_plan_from_schedule(current_week_schedule)
		_emit_weekly_plan_update()
	if mvp_mode_enabled:
		week_started.emit(current_week, current_week_schedule)


## ============================================================================
## MVP HELPER METHODS (Phase 7B Weekly Loop)
## ============================================================================


func enable_mvp_mode(start_year: int = 1, start_week: int = 1) -> void:
	mvp_mode_enabled = true
	mvp_completed = false
	current_year = start_year
	current_week = start_week
	current_day = 1
	current_day_of_week = 0
	week_started.emit(current_week, null)


func get_current_mvp_day_name() -> String:
	return MVP_DAY_NAMES[current_day_of_week]


func advance_day_mvp() -> Dictionary:
	if not mvp_mode_enabled:
		push_warning("[DateManager] MVP mode not enabled; call enable_mvp_mode() first")
		return {}

	if mvp_completed:
		return {
			"completed": true,
			"year": current_year,
			"week": current_week,
			"day_of_week": current_day_of_week,
			"day_name": MVP_DAY_NAMES[current_day_of_week]
		}

	current_day += 1
	current_day_of_week = (current_day_of_week + 1) % 7

	if current_day_of_week == 0:
		current_week += 1
		if current_week > MVP_MAX_WEEKS_PER_YEAR:
			current_week = MVP_MAX_WEEKS_PER_YEAR
			mvp_completed = true
		else:
			week_started.emit(current_week, null)

	return {
		"completed": mvp_completed,
		"year": current_year,
		"week": current_week,
		"day_of_week": current_day_of_week,
		"day_name": MVP_DAY_NAMES[current_day_of_week]
	}


func advance_week_mvp() -> Dictionary:
	if not mvp_mode_enabled:
		push_warning("[DateManager] MVP mode not enabled; call enable_mvp_mode() first")
		return {}

	if mvp_completed:
		return {"completed": true, "year": current_year, "week": current_week}

	current_week += 1
	current_day_of_week = 0
	if current_week > MVP_MAX_WEEKS_PER_YEAR:
		current_week = MVP_MAX_WEEKS_PER_YEAR
		mvp_completed = true
	else:
		week_started.emit(current_week, null)

	return {"completed": mvp_completed, "year": current_year, "week": current_week}


func reset_week_for_mvp() -> void:
	current_day_of_week = 0
	if mvp_mode_enabled and not mvp_completed:
		week_started.emit(current_week, null)


func is_mvp_completed() -> bool:
	return mvp_completed
