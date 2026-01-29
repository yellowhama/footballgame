extends Node

## CoachEvaluationManager.gd
##
## 감독 평가 시스템 (監督評価) - Phase 1 of Gameplay Upgrade Plan
##
## Manages coach evaluation based on team training participation and match performance.
## Determines player's match eligibility (NONE/BENCH/STARTER).
##
## Based on パワフルプロ野球 Success Mode mechanics.

# ============================================================
# SIGNALS
# ============================================================

## Emitted when evaluation value changes
## @param new_value: New evaluation score (0-100)
## @param reason: Reason for the change (e.g., "팀훈련 참여", "경기 좋은 활약")
signal evaluation_changed(new_value: int, reason: String)

## Emitted when match eligibility status changes
## @param status: New eligibility status (NONE/BENCH/STARTER)
signal match_eligibility_changed(status: EligibilityStatus)

# ============================================================
# ENUMS
# ============================================================

## Match eligibility status based on evaluation score
enum EligibilityStatus { NONE = 0, BENCH = 1, STARTER = 2 }  ## 0-19: 출전 불가 (Cannot play)  ## 20-39: 벤치 입장만 (Bench only)  ## 40+: 선발 출전 (Starting lineup)

# ============================================================
# STATE VARIABLES
# ============================================================

## Current coach evaluation score (0-100)
## Default: 30 (allows bench participation)
var coach_evaluation: int = 30:
	set(value):
		coach_evaluation = clampi(value, 0, 100)
		var old_status = current_eligibility_status
		current_eligibility_status = _calculate_eligibility_status()
		if old_status != current_eligibility_status:
			match_eligibility_changed.emit(current_eligibility_status)

## Current eligibility status
var current_eligibility_status: EligibilityStatus = EligibilityStatus.BENCH

# ============================================================
# LIFECYCLE
# ============================================================


func _ready() -> void:
	# Initialize eligibility status
	current_eligibility_status = _calculate_eligibility_status()
	print(
		(
			"[CoachEvaluationManager] Initialized with evaluation: %d, status: %s"
			% [coach_evaluation, get_eligibility_name(current_eligibility_status)]
		)
	)


# ============================================================
# PUBLIC API - TRAINING EVENTS
# ============================================================


## Called when player attends team training
## @param performance_score: Training performance (0-100)
func on_team_training_attended(performance_score: int) -> void:
	var gain = int(performance_score / 10.0)  # 0-10 gain
	change_evaluation(gain, "팀훈련 참여")


## Called when player skips team training
func on_team_training_skipped() -> void:
	change_evaluation(-15, "팀훈련 불참")


# ============================================================
# PUBLIC API - MATCH EVENTS
# ============================================================


## Called when player performs well in a match
func on_match_good_performance() -> void:
	change_evaluation(8, "경기 좋은 활약")


## Called when player performs poorly in a match
func on_match_poor_performance() -> void:
	change_evaluation(-5, "경기 부진")


## Called when player wins a match
func on_match_victory() -> void:
	change_evaluation(3, "경기 승리")


## Called when player loses a match
func on_match_defeat() -> void:
	change_evaluation(-2, "경기 패배")


# ============================================================
# PUBLIC API - QUERIES
# ============================================================


## Returns current match eligibility status
func get_match_eligibility() -> EligibilityStatus:
	return current_eligibility_status


## Returns current evaluation score
func get_evaluation() -> int:
	return coach_evaluation


## Checks if player can start in matches
func can_start_match() -> bool:
	return current_eligibility_status == EligibilityStatus.STARTER


## Checks if player can be on the bench
func can_be_on_bench() -> bool:
	return current_eligibility_status >= EligibilityStatus.BENCH


## Checks if player can participate at all
func can_participate() -> bool:
	return current_eligibility_status > EligibilityStatus.NONE


# ============================================================
# CORE FUNCTIONS
# ============================================================


## Changes evaluation score and emits signals
## @param delta: Amount to change (+/-)
## @param reason: Reason for the change
func change_evaluation(delta: int, reason: String) -> void:
	var old_value = coach_evaluation
	coach_evaluation += delta  # Triggers setter with clamping

	if old_value != coach_evaluation:
		evaluation_changed.emit(coach_evaluation, reason)

		# Log the change
		var change_text = "+%d" % delta if delta > 0 else "%d" % delta
		print("[CoachEvaluationManager] %s: %d → %d (%s)" % [reason, old_value, coach_evaluation, change_text])


## Calculates eligibility status from current evaluation
func _calculate_eligibility_status() -> EligibilityStatus:
	if coach_evaluation < 20:
		return EligibilityStatus.NONE
	elif coach_evaluation < 40:
		return EligibilityStatus.BENCH
	else:
		return EligibilityStatus.STARTER


# ============================================================
# UI HELPER FUNCTIONS
# ============================================================


## Returns Korean name for eligibility status
func get_eligibility_name(status: EligibilityStatus) -> String:
	match status:
		EligibilityStatus.NONE:
			return "출전 불가"
		EligibilityStatus.BENCH:
			return "벤치"
		EligibilityStatus.STARTER:
			return "선발"
		_:
			return "알 수 없음"


## Returns color for UI display based on status
func get_eligibility_color(status: EligibilityStatus) -> Color:
	match status:
		EligibilityStatus.NONE:
			return Color(0.8, 0.2, 0.2)  # Red
		EligibilityStatus.BENCH:
			return Color(0.8, 0.6, 0.2)  # Yellow
		EligibilityStatus.STARTER:
			return Color(0.2, 0.8, 0.2)  # Green
		_:
			return Color(0.5, 0.5, 0.5)  # Gray


## Returns formatted evaluation string for UI
## Example: "감독 평가: 35/100 (벤치)"
func get_evaluation_display() -> String:
	return "감독 평가: %d/100 (%s)" % [coach_evaluation, get_eligibility_name(current_eligibility_status)]


## Returns description of current status
func get_status_description() -> String:
	match current_eligibility_status:
		EligibilityStatus.NONE:
			return "감독 평가가 너무 낮아 출전할 수 없습니다. 팀훈련에 참여하세요!"
		EligibilityStatus.BENCH:
			return "벤치에 입장할 수 있습니다. 평가 40점 이상이면 선발 출전 가능!"
		EligibilityStatus.STARTER:
			return "선발 라인업에 들어갈 수 있습니다!"
		_:
			return ""


## Returns points needed to reach next threshold
func get_points_to_next_threshold() -> int:
	match current_eligibility_status:
		EligibilityStatus.NONE:
			return 20 - coach_evaluation  # To reach BENCH
		EligibilityStatus.BENCH:
			return 40 - coach_evaluation  # To reach STARTER
		EligibilityStatus.STARTER:
			return 0  # Already at max
		_:
			return 0


# ============================================================
# DEBUG FUNCTIONS
# ============================================================


## Sets evaluation directly (for testing)
func debug_set_evaluation(value: int) -> void:
	if OS.is_debug_build():
		coach_evaluation = value
		print("[CoachEvaluationManager] DEBUG: Set evaluation to %d" % value)


## Prints current state
func debug_print_state() -> void:
	if OS.is_debug_build():
		print("=== CoachEvaluationManager State ===")
		print("Evaluation: %d/100" % coach_evaluation)
		print("Status: %s" % get_eligibility_name(current_eligibility_status))
		print("Can start: %s" % can_start_match())
		print("Can bench: %s" % can_be_on_bench())
		print("Points to next: %d" % get_points_to_next_threshold())
		print("===================================")
