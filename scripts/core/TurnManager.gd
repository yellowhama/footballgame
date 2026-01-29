extends Node
## TurnManager - 48-Turn System for Academy Mode
##
## Maps the 156-week calendar to 48 key decision turns where the player makes
## strategic choices (training, coach cards, events). Integrates Uma Musume-style
## turn-based progression with the existing weekly calendar system.
##
## Architecture:
## - 3 years × 16 turns/year = 48 total turns
## - Each turn maps to ~3 weeks of game time
## - Decision turns: Player selects training/coach cards
## - Auto-progression turns: Match weeks, rest periods, events
##
## Integration with GameManager:
## - GameManager handles weekly progression (156 weeks)
## - TurnManager overlays turn-based decision points
## - Turn completion triggers weekly progression until next decision point

signal turn_started(turn_info: Dictionary)
signal turn_completed(turn_number: int, weeks_progressed: int)
signal decision_required(turn_info: Dictionary, available_actions: Array)
signal year_completed(year: int)
signal academy_completed

## Turn Types
enum TurnType { DECISION, MATCH, EVENT, REST, CAMP, MILESTONE }  # 플레이어가 훈련/코치카드 선택  # 경기 주 (자동 진행)  # 이벤트 발생 (선택지 있을 수 있음)  # 휴식/회복 주  # 캠프 기간 (특수 훈련)  # 주요 이정표 (승급 심사 등)

## Turn Data
const TURNS_PER_YEAR: int = 16
const TOTAL_TURNS: int = 48  # 3 years
const TOTAL_WEEKS: int = 156  # 3 years × 52 weeks
const DECISION_TURNS_PER_YEAR: int = 10  # 연간 실제 결정 턴 수
const AUTO_TURNS_PER_YEAR: int = 6  # 연간 자동 진행 턴 수

## Current State
var current_turn: int = 1
var current_year: int = 1
var turn_schedule: Array[Dictionary] = []
var completed_turns: Array[int] = []

## Week Mapping
var turn_to_week_map: Dictionary = {}  # {turn_number: start_week}
var week_to_turn_map: Dictionary = {}  # {week_number: turn_number}


func _ready() -> void:
	print("[TurnManager] Initializing 48-turn system...")
	generate_turn_schedule()
	print("[TurnManager] ✅ Turn schedule generated: %d turns over %d weeks" % [turn_schedule.size(), TOTAL_WEEKS])


## Generate the complete 48-turn schedule
func generate_turn_schedule() -> void:
	turn_schedule.clear()
	turn_to_week_map.clear()
	week_to_turn_map.clear()

	var current_week: int = 1

	for year in range(1, 4):  # Years 1-3
		var turns_this_year: Array[Dictionary] = _generate_year_schedule(year, current_week)
		turn_schedule.append_array(turns_this_year)

		# Update current_week for next year
		if turns_this_year.size() > 0:
			var last_turn = turns_this_year[-1]
			current_week = last_turn.end_week + 1

	# Build mappings
	for turn_data in turn_schedule:
		var turn_num: int = turn_data.turn_number
		var start_week: int = turn_data.start_week

		turn_to_week_map[turn_num] = start_week

		# Map all weeks in this turn's range to the turn number
		for week in range(turn_data.start_week, turn_data.end_week + 1):
			week_to_turn_map[week] = turn_num


## Generate schedule for a single year (16 turns)
func _generate_year_schedule(year: int, start_week: int) -> Array[Dictionary]:
	var year_turns: Array[Dictionary] = []
	var week_offset: int = start_week

	# Year structure (16 turns):
	# - 12 season turns (weeks 1-36): mix of decision/match/event
	# - 2 camp turns (weeks 37-44): summer camp
	# - 2 vacation turns (weeks 45-52): rest and preparation

	var turn_templates: Array[Dictionary] = [
		# Season Phase 1 (weeks 1-12) - 4 turns
		{"type": TurnType.DECISION, "weeks": 3, "label": "개인훈련 선택 1"},
		{"type": TurnType.MATCH, "weeks": 2, "label": "첫 경기"},
		{"type": TurnType.DECISION, "weeks": 3, "label": "개인훈련 선택 2"},
		{"type": TurnType.EVENT, "weeks": 4, "label": "코치 이벤트"},
		# Season Phase 2 (weeks 13-24) - 4 turns
		{"type": TurnType.DECISION, "weeks": 3, "label": "집중 훈련 1"},
		{"type": TurnType.MATCH, "weeks": 2, "label": "중요 경기"},
		{"type": TurnType.DECISION, "weeks": 3, "label": "집중 훈련 2"},
		{"type": TurnType.EVENT, "weeks": 4, "label": "특별 관계 이벤트"},
		# Season Phase 3 (weeks 25-36) - 4 turns
		{"type": TurnType.DECISION, "weeks": 3, "label": "최종 훈련 1"},
		{"type": TurnType.MATCH, "weeks": 2, "label": "시즌 마무리 경기"},
		{"type": TurnType.DECISION, "weeks": 3, "label": "최종 훈련 2"},
		{"type": TurnType.MILESTONE, "weeks": 4, "label": "시즌 평가"},
		# Summer Camp (weeks 37-44) - 2 turns
		{"type": TurnType.CAMP, "weeks": 4, "label": "여름 캠프 전반"},
		{"type": TurnType.CAMP, "weeks": 4, "label": "여름 캠프 후반"},
		# Vacation (weeks 45-52) - 2 turns
		{"type": TurnType.REST, "weeks": 4, "label": "휴가 및 개인 정비"},
		{"type": TurnType.EVENT, "weeks": 4, "label": "휴가 이벤트"}
	]

	var base_turn_number: int = (year - 1) * TURNS_PER_YEAR + 1

	for i in range(turn_templates.size()):
		var template: Dictionary = turn_templates[i]
		var turn_number: int = base_turn_number + i
		var weeks_duration: int = template.weeks
		var end_week: int = week_offset + weeks_duration - 1

		var turn_data: Dictionary = {
			"turn_number": turn_number,
			"year": year,
			"turn_in_year": i + 1,
			"type": template.type,
			"label": template.label,
			"start_week": week_offset,
			"end_week": end_week,
			"weeks_duration": weeks_duration,
			"completed": false,
			"decisions_made": {},
			"results": {}
		}

		year_turns.append(turn_data)
		week_offset = end_week + 1

	return year_turns


## Start a turn and emit signals for UI/game logic
func start_turn(turn_number: int) -> void:
	if turn_number < 1 or turn_number > TOTAL_TURNS:
		push_error("[TurnManager] Invalid turn number: %d" % turn_number)
		return

	current_turn = turn_number
	var turn_info: Dictionary = get_turn_info(turn_number)
	current_year = turn_info.year

	print(
		(
			"[TurnManager] 🎯 Starting Turn %d/%d (Year %d, Week %d-%d): %s"
			% [turn_number, TOTAL_TURNS, turn_info.year, turn_info.start_week, turn_info.end_week, turn_info.label]
		)
	)

	turn_started.emit(turn_info)

	# If this is a decision turn, emit decision_required
	if turn_info.type == TurnType.DECISION:
		var actions: Array = _get_available_actions(turn_info)
		decision_required.emit(turn_info, actions)
	else:
		# Auto-progression turns can be handled automatically
		print("[TurnManager] ⏩ Auto-progression turn, advancing...")


## Complete the current turn and progress to next
func complete_turn(results: Dictionary = {}) -> void:
	if current_turn > TOTAL_TURNS:
		push_error("[TurnManager] No active turn to complete")
		return

	var turn_info: Dictionary = get_turn_info(current_turn)
	turn_info.completed = true
	turn_info.results = results
	completed_turns.append(current_turn)

	var weeks_progressed: int = turn_info.weeks_duration

	print("[TurnManager] ✅ Turn %d completed (%d weeks progressed)" % [current_turn, weeks_progressed])
	turn_completed.emit(current_turn, weeks_progressed)

	# Check for year completion
	if turn_info.turn_in_year == TURNS_PER_YEAR:
		print("[TurnManager] 🎉 Year %d completed!" % turn_info.year)
		year_completed.emit(turn_info.year)

	# Check for academy completion
	if current_turn >= TOTAL_TURNS:
		print("[TurnManager] 🏆 Academy journey completed!")
		academy_completed.emit()
		return

	# Advance to next turn
	current_turn += 1
	start_turn(current_turn)


## Get information about a specific turn
func get_turn_info(turn_number: int) -> Dictionary:
	if turn_number < 1 or turn_number > turn_schedule.size():
		return {}
	return turn_schedule[turn_number - 1]


## Get current turn information
func get_current_turn_info() -> Dictionary:
	return get_turn_info(current_turn)


## Get which turn a specific week belongs to
func get_turn_for_week(week_number: int) -> int:
	return week_to_turn_map.get(week_number, -1)


## Get the starting week for a turn
func get_week_for_turn(turn_number: int) -> int:
	return turn_to_week_map.get(turn_number, -1)


## Check if current turn requires player decision
func is_decision_turn() -> bool:
	var turn_info: Dictionary = get_current_turn_info()
	return turn_info.get("type", -1) == TurnType.DECISION


## Get available actions for a turn (to be expanded with coach cards, training options)
func _get_available_actions(turn_info: Dictionary) -> Array:
	var actions: Array = []

	match turn_info.type:
		TurnType.DECISION:
			# Core training actions (will integrate with CoachCardSystem later)
			actions.append(
				{
					"id": "training_technical",
					"label": "기술 훈련",
					"category": "training",
					"description": "패스, 드리블, 슈팅 능력 향상"
				}
			)
			actions.append(
				{"id": "training_physical", "label": "체력 훈련", "category": "training", "description": "스피드, 스태미나, 파워 향상"}
			)
			actions.append(
				{"id": "training_mental", "label": "멘탈 훈련", "category": "training", "description": "집중력, 결단력, 비전 향상"}
			)
			actions.append({"id": "rest", "label": "휴식", "category": "recovery", "description": "체력 회복 및 부상 예방"})

		TurnType.EVENT:
			# Event choices (placeholder)
			actions.append({"id": "event_choice_a", "label": "선택지 A", "category": "event"})
			actions.append({"id": "event_choice_b", "label": "선택지 B", "category": "event"})

		TurnType.CAMP:
			# Camp special training options
			actions.append(
				{"id": "camp_intensive", "label": "집중 특훈", "category": "camp", "description": "높은 성장률, 높은 체력 소모"}
			)
			actions.append(
				{"id": "camp_balanced", "label": "균형 훈련", "category": "camp", "description": "안정적 성장, 적당한 체력 소모"}
			)

	return actions


## Get overall progress statistics
func get_progress_stats() -> Dictionary:
	return {
		"current_turn": current_turn,
		"total_turns": TOTAL_TURNS,
		"current_year": current_year,
		"total_years": 3,
		"completed_turns": completed_turns.size(),
		"remaining_turns": TOTAL_TURNS - completed_turns.size(),
		"progress_percentage": (completed_turns.size() * 100.0) / TOTAL_TURNS,
		"current_turn_info": get_current_turn_info()
	}


## Reset for new game
func reset() -> void:
	current_turn = 1
	current_year = 1
	completed_turns.clear()

	# Mark all turns as not completed
	for turn_data in turn_schedule:
		turn_data.completed = false
		turn_data.decisions_made = {}
		turn_data.results = {}

	print("[TurnManager] 🔄 Reset to turn 1")


## Debug: Print schedule overview
func print_schedule() -> void:
	print("\n[TurnManager] 📋 48-Turn Schedule:")
	print("=".repeat(80))

	for year in range(1, 4):
		print("\n🗓️ YEAR %d" % year)
		print("-".repeat(80))

		var year_turns: Array[Dictionary] = turn_schedule.filter(func(t): return t.year == year)

		for turn_data in year_turns:
			var type_icon: String = _get_type_icon(turn_data.type)
			var status: String = "✅" if turn_data.completed else "⏳"

			print(
				(
					"%s Turn %2d | Week %2d-%2d (%d weeks) | %s %s"
					% [
						status,
						turn_data.turn_number,
						turn_data.start_week,
						turn_data.end_week,
						turn_data.weeks_duration,
						type_icon,
						turn_data.label
					]
				)
			)

	print("\n" + "=".repeat(80))


func _get_type_icon(type: TurnType) -> String:
	match type:
		TurnType.DECISION:
			return "🎯"
		TurnType.MATCH:
			return "⚽"
		TurnType.EVENT:
			return "📖"
		TurnType.REST:
			return "😴"
		TurnType.CAMP:
			return "🏕️"
		TurnType.MILESTONE:
			return "🏆"
	return "❓"
