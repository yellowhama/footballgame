## WeekSchedule.gd
## Weekly schedule data structure for Academy Mode
## Determines daily activities based on match fixtures and period type
extends Resource
class_name WeekSchedule

## ============================================================================
## ENUMS
## ============================================================================

## Week types based on match scheduling
enum WeekType { MATCH_WEEK_SATURDAY, MATCH_WEEK_MIDWEEK, MATCH_WEEK_BOTH, NO_MATCH_WEEK }  ## Match on Saturday only  ## Match on Wednesday only  ## Matches on both Wednesday and Saturday  ## No matches scheduled (training week)

## Period types (from GameManager)
enum Period { SEASON, CAMP, VACATION }  ## Regular season - full activities  ## Training camp - intensive training  ## Vacation period - minimal activities

## Day of week enumeration
enum DayOfWeek { MONDAY = 0, TUESDAY = 1, WEDNESDAY = 2, THURSDAY = 3, FRIDAY = 4, SATURDAY = 5, SUNDAY = 6 }

## ============================================================================
## CORE PROPERTIES
## ============================================================================

## Week identification
@export var week_number: int = 1  ## 1-38 per year
@export var year: int = 1  ## 1-3 (U16/U17/U18)

## Week classification
@export var week_type: WeekType = WeekType.NO_MATCH_WEEK
@export var period: Period = Period.SEASON

## Daily activity schedules
@export var training_days: Array[int] = []  ## [0,1,2,3,4] (Mon-Fri typical)
@export var match_days: Array[int] = []  ## [5] or [2] or [2,5]
@export var match_prep_days: Array[int] = []  ## [4] or [1] or [1,4]
@export var recovery_days: Array[int] = []  ## [6] or [3] or [3,6]
@export var day_plans: Dictionary = {}  ## day -> metadata (type/mode/focus)

## Match fixtures for this week
@export var fixtures: Array = []  ## Array of MatchFixture objects

## ============================================================================
## INITIALIZATION
## ============================================================================


func _init(
	p_week_number: int = 1,
	p_year: int = 1,
	p_week_type: WeekType = WeekType.NO_MATCH_WEEK,
	p_period: Period = Period.SEASON
) -> void:
	week_number = p_week_number
	year = p_year
	week_type = p_week_type
	period = p_period

	# Generate default schedule based on week type
	_generate_default_schedule()


## ============================================================================
## SCHEDULE GENERATION
## ============================================================================


## Generate default daily schedule based on week type
func _generate_default_schedule() -> void:
	# Clear existing schedules
	training_days.clear()
	match_days.clear()
	match_prep_days.clear()
	recovery_days.clear()
	day_plans.clear()

	if period == Period.VACATION:
		# Vacation: all recovery
		recovery_days = [0, 1, 2, 3, 4, 5, 6]
		return

	match week_type:
		WeekType.MATCH_WEEK_SATURDAY:
			# Saturday match week
			# Mon-Thu: Training
			# Fri: Match Prep
			# Sat: Match
			# Sun: Recovery
			training_days = [DayOfWeek.MONDAY, DayOfWeek.TUESDAY, DayOfWeek.WEDNESDAY, DayOfWeek.THURSDAY]
			match_prep_days = [DayOfWeek.FRIDAY]
			match_days = [DayOfWeek.SATURDAY]
			recovery_days = [DayOfWeek.SUNDAY]

		WeekType.MATCH_WEEK_MIDWEEK:
			# Wednesday match week
			# Mon: Training
			# Tue: Match Prep
			# Wed: Match
			# Thu: Recovery
			# Fri: Training
			# Sat-Sun: Light/Recovery
			training_days = [DayOfWeek.MONDAY, DayOfWeek.FRIDAY]
			match_prep_days = [DayOfWeek.TUESDAY]
			match_days = [DayOfWeek.WEDNESDAY]
			recovery_days = [DayOfWeek.THURSDAY, DayOfWeek.SATURDAY, DayOfWeek.SUNDAY]

		WeekType.MATCH_WEEK_BOTH:
			# Both matches week
			# Mon: Training
			# Tue: Match Prep (Wed)
			# Wed: Match
			# Thu: Recovery
			# Fri: Match Prep (Sat)
			# Sat: Match
			# Sun: Recovery
			training_days = [DayOfWeek.MONDAY]
			match_prep_days = [DayOfWeek.TUESDAY, DayOfWeek.FRIDAY]
			match_days = [DayOfWeek.WEDNESDAY, DayOfWeek.SATURDAY]
			recovery_days = [DayOfWeek.THURSDAY, DayOfWeek.SUNDAY]

		WeekType.NO_MATCH_WEEK:
			# No match week (Camp or regular training)
			# Mon-Fri: Training (5 days)
			# Sat: Light training or event
			# Sun: Recovery
			if period == Period.CAMP:
				# Camp: 6 training days
				training_days = [
					DayOfWeek.MONDAY,
					DayOfWeek.TUESDAY,
					DayOfWeek.WEDNESDAY,
					DayOfWeek.THURSDAY,
					DayOfWeek.FRIDAY,
					DayOfWeek.SATURDAY
				]
				recovery_days = [DayOfWeek.SUNDAY]
			else:
				# Regular: 5 training days
				training_days = [
					DayOfWeek.MONDAY, DayOfWeek.TUESDAY, DayOfWeek.WEDNESDAY, DayOfWeek.THURSDAY, DayOfWeek.FRIDAY
				]
				recovery_days = [DayOfWeek.SATURDAY, DayOfWeek.SUNDAY]

	for day in range(7):
		var day_type := "rest"
		if day in match_days:
			day_type = "match"
		elif day in match_prep_days:
			day_type = "prep"
		elif day in training_days:
			day_type = "training"
		elif day in recovery_days:
			day_type = "recovery"
		_set_default_day_plan(day, day_type)


func _set_default_day_plan(day_of_week: int, day_type: String) -> void:
	var base := {"type": day_type, "mode": "team" if day_type in ["match", "prep"] else "personal", "focus": ""}
	match day_type:
		"training":
			base.focus = "피지컬"
		"prep":
			base.focus = "전술"
		"match":
			base.focus = "경기"
		"recovery":
			base.focus = "회복"
		_:
			base.focus = "휴식"
	set_day_plan(day_of_week, base)


func set_day_plan(day_of_week: int, plan: Dictionary) -> void:
	day_plans[day_of_week] = plan.duplicate(true) if plan else {}


func get_day_plan(day_of_week: int) -> Dictionary:
	return day_plans.get(day_of_week, {}).duplicate(true)


## ============================================================================
## QUERY METHODS
## ============================================================================


## Check if given day is a training day
func is_training_day(day_of_week: int) -> bool:
	return day_of_week in training_days


## Check if given day has a match
func has_match(day_of_week: int) -> bool:
	return day_of_week in match_days


## Check if given day is match prep day
func is_match_prep_day(day_of_week: int) -> bool:
	return day_of_week in match_prep_days


## Check if given day is recovery day
func is_recovery_day(day_of_week: int) -> bool:
	return day_of_week in recovery_days


## Get fixture for specific day (if exists)
func get_fixture_for_day(day_of_week: int):  ## Returns MatchFixture or null
	for fixture in fixtures:
		if fixture.scheduled_day_of_week == day_of_week:
			return fixture
	return null


## Get all fixtures for this week
func get_fixtures() -> Array:  ## Returns Array of MatchFixture
	return fixtures


## Get total training days count
func get_training_count() -> int:
	return training_days.size()


## Get total match count
func get_match_count() -> int:
	return match_days.size()


## ============================================================================
## SCHEDULE MODIFICATION
## ============================================================================


## Add a fixture to this week's schedule
func add_fixture(fixture) -> void:  ## fixture: MatchFixture
	if fixture not in fixtures:
		fixtures.append(fixture)
		_update_week_type()


## Remove a fixture from schedule
func remove_fixture(fixture) -> void:  ## fixture: MatchFixture
	fixtures.erase(fixture)
	_update_week_type()


## Update week type based on current fixtures
func _update_week_type() -> void:
	var has_saturday = fixtures.any(func(f): return f.scheduled_day_of_week == DayOfWeek.SATURDAY)
	var has_midweek = fixtures.any(func(f): return f.scheduled_day_of_week == DayOfWeek.WEDNESDAY)

	if has_saturday and has_midweek:
		week_type = WeekType.MATCH_WEEK_BOTH
	elif has_saturday:
		week_type = WeekType.MATCH_WEEK_SATURDAY
	elif has_midweek:
		week_type = WeekType.MATCH_WEEK_MIDWEEK
	else:
		week_type = WeekType.NO_MATCH_WEEK

	# Regenerate schedule
	_generate_default_schedule()


## ============================================================================
## SERIALIZATION
## ============================================================================


## Convert to dictionary for save system
func to_dict() -> Dictionary:
	return {
		"week_number": week_number,
		"year": year,
		"week_type": week_type,
		"period": period,
		"training_days": training_days,
		"match_days": match_days,
		"match_prep_days": match_prep_days,
		"recovery_days": recovery_days,
		"fixtures": fixtures.map(func(f): return f.to_dict())
	}


## Create from dictionary (for load system)
static func from_dict(data: Dictionary):  # Returns WeekSchedule
	# Static functions cannot use class_name for instantiation - use load().new()
	var schedule = load("res://scripts/academy/WeekSchedule.gd").new()

	schedule.week_number = data.get("week_number", 1)
	schedule.year = data.get("year", 1)
	schedule.week_type = data.get("week_type", WeekType.NO_MATCH_WEEK)
	schedule.period = data.get("period", Period.SEASON)

	schedule.training_days = data.get("training_days", [])
	schedule.match_days = data.get("match_days", [])
	schedule.match_prep_days = data.get("match_prep_days", [])
	schedule.recovery_days = data.get("recovery_days", [])

	# Restore fixtures
	var fixtures_data = data.get("fixtures", [])
	for fixture_dict in fixtures_data:
		# Cannot use MatchFixture class_name in static context - use load().from_dict()
		var MatchFixtureScript = load("res://scripts/academy/MatchFixture.gd")
		var fixture = MatchFixtureScript.from_dict(fixture_dict)
		schedule.fixtures.append(fixture)

	return schedule


## ============================================================================
## DEBUG & DISPLAY
## ============================================================================


## Get human-readable week type name
func get_week_type_name() -> String:
	match week_type:
		WeekType.MATCH_WEEK_SATURDAY:
			return "Saturday Match Week"
		WeekType.MATCH_WEEK_MIDWEEK:
			return "Midweek Match Week"
		WeekType.MATCH_WEEK_BOTH:
			return "Double Match Week"
		WeekType.NO_MATCH_WEEK:
			return "Training Week"
		_:
			return "Unknown"


## Get human-readable period name
func get_period_name() -> String:
	match period:
		Period.SEASON:
			return "Season"
		Period.CAMP:
			return "Training Camp"
		Period.VACATION:
			return "Vacation"
		_:
			return "Unknown"


## Get day name from int
static func get_day_name(day_of_week: int) -> String:
	match day_of_week:
		DayOfWeek.MONDAY:
			return "Monday"
		DayOfWeek.TUESDAY:
			return "Tuesday"
		DayOfWeek.WEDNESDAY:
			return "Wednesday"
		DayOfWeek.THURSDAY:
			return "Thursday"
		DayOfWeek.FRIDAY:
			return "Friday"
		DayOfWeek.SATURDAY:
			return "Saturday"
		DayOfWeek.SUNDAY:
			return "Sunday"
		_:
			return "Invalid"


## Get week summary for debugging
func get_week_summary() -> String:
	var summary = "Week %d (Year %d) - %s - %s\n" % [week_number, year, get_week_type_name(), get_period_name()]

	summary += "Training Days: %s\n" % _format_days(training_days)
	summary += "Match Days: %s\n" % _format_days(match_days)
	summary += "Match Prep Days: %s\n" % _format_days(match_prep_days)
	summary += "Recovery Days: %s\n" % _format_days(recovery_days)
	summary += "Fixtures: %d\n" % fixtures.size()

	for fixture in fixtures:
		summary += "  - %s\n" % fixture.get_match_description()

	return summary


## Format day array as readable string
func _format_days(days: Array[int]) -> String:
	if days.is_empty():
		return "None"

	var day_names = days.map(func(d): return get_day_name(d))
	return ", ".join(day_names)


## ============================================================================
## VALIDATION
## ============================================================================


## Validate schedule consistency
func validate() -> Dictionary:
	var errors = []
	var warnings = []

	# Check for overlapping activities
	var all_days = {}

	for day in training_days:
		if day in all_days:
			errors.append("Day %s assigned to multiple activities" % get_day_name(day))
		all_days[day] = "training"

	for day in match_days:
		if day in all_days:
			errors.append("Day %s assigned to multiple activities (match + %s)" % [get_day_name(day), all_days[day]])
		all_days[day] = "match"

	for day in match_prep_days:
		if day in all_days:
			errors.append("Day %s assigned to multiple activities (prep + %s)" % [get_day_name(day), all_days[day]])
		all_days[day] = "prep"

	for day in recovery_days:
		if day in all_days:
			warnings.append("Day %s assigned to both activity and recovery" % get_day_name(day))
		all_days[day] = "recovery"

	# Check fixture consistency
	for fixture in fixtures:
		if fixture.scheduled_day_of_week not in match_days:
			errors.append("Fixture on %s but day not in match_days" % get_day_name(fixture.scheduled_day_of_week))

	# Check week coverage
	if all_days.size() < 7:
		warnings.append("Not all days of week assigned activities")

	return {"valid": errors.is_empty(), "errors": errors, "warnings": warnings}
