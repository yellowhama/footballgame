extends Node

## DecisionTracker - Phase 24: Career Ending System
##
## Purpose: Log all user decisions with full context to enable decision-based career endings
## SSOT Requirement: "Endings are generated from decisions, not selected from a fixed list"
##
## Logs:
## - Training choices (Intense vs Light, despite condition)
## - Match tactics (Aggressive vs Defensive)
## - Story event choices (A/B/C options)
## - Rest/Outing decisions
##
## Each decision includes:
## - What was chosen
## - What alternatives were available
## - Why it was chosen (context: condition, week, CA, etc.)
## - What happened (outcome: CA gain, condition loss, injury, etc.)

## Decision log: Array of decision entries
## Each entry: {week, year, type, choice, alternatives, context, outcome, is_key_moment, narrative}
var decision_log: Array = []

## Key moments cache (top 8 most significant decisions)
var _key_moments_cache: Array = []
var _cache_dirty: bool = true

## Signal emitted when a decision is logged (for debugging/analytics)
signal decision_logged(type: String, choice: String)

## Signal emitted when key moments are recalculated
signal key_moments_updated(moments: Array)


## Log a decision with full context
##
## @param type: Decision type ("training", "match_tactic", "event", "rest", "outing")
## @param choice: What was chosen (e.g., "Intense Training", "Ultra Attacking")
## @param alternatives: What else could have been chosen (Array of Strings)
## @param context: Why this decision was made (Dictionary with condition, week, ca, etc.)
## @param outcome: What happened as a result (Dictionary with ca_gain, condition_loss, etc.)
func log_decision(
	type: String, choice: String, alternatives: Array, context: Dictionary, outcome: Dictionary = {}
) -> void:
	var week = _get_current_week()
	var year = _get_current_year()

	var entry = {
		"week": week,
		"year": year,
		"type": type,
		"choice": choice,
		"alternatives": alternatives.duplicate(),
		"context": context.duplicate(true),
		"outcome": outcome.duplicate(true),
		"is_key_moment": false,  # Will be flagged later during analysis
		"timestamp": Time.get_unix_time_from_system(),
		"narrative": ""  # Will be generated later
	}

	# Generate basic narrative
	entry["narrative"] = _generate_decision_narrative(entry)

	decision_log.append(entry)
	_cache_dirty = true

	decision_logged.emit(type, choice)

	if OS.is_debug_build():
		print("[DecisionTracker] Logged %s decision: '%s' (Week %d, Year %d)" % [type, choice, week, year])
		if outcome.has("ca_gain") and outcome.ca_gain > 0:
			print("  → CA +%d" % outcome.ca_gain)


## Get all decisions of a specific type
##
## @param type: Decision type to filter ("training", "event", etc.)
## @return: Array of decision entries matching the type
func get_decisions_by_type(type: String) -> Array:
	return decision_log.filter(func(d): return d.type == type)


## Get all decisions from a specific week range
##
## @param start_week: Starting week (inclusive)
## @param end_week: Ending week (inclusive)
## @return: Array of decision entries in the range
func get_decisions_in_range(start_week: int, end_week: int) -> Array:
	return decision_log.filter(func(d): return d.week >= start_week and d.week <= end_week)


## Get all decisions from a specific year
##
## @param year: Year to filter (1, 2, or 3)
## @return: Array of decision entries from that year
func get_decisions_by_year(year: int) -> Array:
	return decision_log.filter(func(d): return d.year == year)


## Get top N key moments (most significant decisions)
##
## Key moments are identified by:
## - High CA gain (>= 3)
## - Critical context (low condition but still trained)
## - Major outcomes (injury, promotion, etc.)
## - High risk/reward decisions
##
## @param top_n: Number of top moments to return (default 8)
## @return: Array of top N decision entries, sorted by significance
func get_key_moments(top_n: int = 8) -> Array:
	if _cache_dirty:
		_recalculate_key_moments()

	return _key_moments_cache.slice(0, min(top_n, _key_moments_cache.size()))


## Analyze all decisions to identify patterns
##
## Patterns:
## - Training tendency: Aggressive (high intensity) vs Conservative (low intensity)
## - Risk-taking: High (train despite low condition) vs Low (always safe)
## - Focus areas: Which attributes trained most
## - Balance score: 0.0 (specialist) ~ 1.0 (all-rounder)
##
## @return: Dictionary with pattern analysis results
func analyze_decision_patterns() -> Dictionary:
	if decision_log.is_empty():
		return _get_default_patterns()

	var training_decisions = get_decisions_by_type("training")

	if training_decisions.is_empty():
		return _get_default_patterns()

	# Count training intensity
	var intense_count = 0
	var total_training = training_decisions.size()

	for decision in training_decisions:
		if _is_intense_training(decision.choice):
			intense_count += 1

	# Classify training tendency
	var training_tendency = "Balanced"
	var intensity_ratio = float(intense_count) / total_training

	if intensity_ratio > 0.6:
		training_tendency = "Aggressive"
	elif intensity_ratio < 0.3:
		training_tendency = "Conservative"

	# Count risky decisions (training despite low condition)
	var risky_count = 0
	for decision in training_decisions:
		var condition = decision.context.get("condition", 100)
		if condition < 50 and _is_intense_training(decision.choice):
			risky_count += 1

	# Classify risk-taking
	var risk_taking = "Balanced"
	var risk_ratio = float(risky_count) / total_training

	if risk_ratio > 0.3:
		risk_taking = "High"
	elif risk_ratio < 0.1:
		risk_taking = "Low"

	# Analyze focus areas (which attributes trained most)
	var focus_areas = _analyze_training_focus(training_decisions)

	# Calculate balance score (0.0 = specialist, 1.0 = all-rounder)
	var balance_score = _calculate_balance_score(training_decisions)

	return {
		"training_tendency": training_tendency,
		"intensity_ratio": intensity_ratio,
		"risk_taking": risk_taking,
		"risk_ratio": risk_ratio,
		"focus_areas": focus_areas,
		"balance_score": balance_score,
		"total_decisions": decision_log.size(),
		"total_training": total_training
	}


## Get all decisions (for debugging/export)
##
## @return: Full decision log array
func get_all_decisions() -> Array:
	return decision_log.duplicate(true)


## Clear all decisions (for testing/reset)
func clear_decisions() -> void:
	decision_log.clear()
	_key_moments_cache.clear()
	_cache_dirty = true

	if OS.is_debug_build():
		print("[DecisionTracker] Decision log cleared")


## Save decision log to dictionary (for SaveManager)
##
## @return: Dictionary with decision_log array
func save_to_dict() -> Dictionary:
	return {"decision_log": decision_log.duplicate(true), "version": 1}


## Load decision log from dictionary (for SaveManager)
##
## @param data: Dictionary from save file
func load_from_dict(data: Dictionary) -> void:
	decision_log = data.get("decision_log", []).duplicate(true)
	_cache_dirty = true

	if OS.is_debug_build():
		print("[DecisionTracker] Loaded %d decisions from save" % decision_log.size())


# ============================================================================
# PRIVATE METHODS
# ============================================================================


## Get current week from DateManager (with fallback)
func _get_current_week() -> int:
	if DateManager:
		return DateManager.current_week
	return 0


## Get current year from DateManager (with fallback)
func _get_current_year() -> int:
	if DateManager:
		return DateManager.current_year
	return 1


## Check if a training choice is "intense" (high intensity)
func _is_intense_training(choice: String) -> bool:
	var intense_keywords = ["Intense", "고강도", "High Intensity", "Hard", "Rigorous"]
	for keyword in intense_keywords:
		if keyword.to_lower() in choice.to_lower():
			return true
	return false


## Generate narrative text for a decision
func _generate_decision_narrative(entry: Dictionary) -> String:
	var narrative = ""

	match entry.type:
		"training":
			var condition = entry.context.get("condition", 100)
			if condition < 50:
				narrative = "체력이 낮았지만 %s 훈련을 선택했습니다." % entry.choice
			else:
				narrative = "%s 훈련을 선택했습니다." % entry.choice

		"event":
			narrative = "이벤트에서 '%s'을(를) 선택했습니다." % entry.choice

		"match_tactic":
			narrative = "경기에서 %s 전술을 사용했습니다." % entry.choice

		"rest":
			narrative = "휴식을 취했습니다."

		"outing":
			narrative = "외출을 선택했습니다."

		_:
			narrative = "%s을(를) 선택했습니다." % entry.choice

	return narrative


## Recalculate key moments cache
func _recalculate_key_moments() -> void:
	var moments = []

	# Identify key moments from decision log
	for decision in decision_log:
		var significance = _calculate_significance(decision)

		if significance > 0.0:
			var moment = decision.duplicate(true)
			moment["significance"] = significance
			moment["is_key_moment"] = true
			moments.append(moment)

	# Sort by significance (highest first)
	moments.sort_custom(func(a, b): return a.significance > b.significance)

	_key_moments_cache = moments
	_cache_dirty = false

	key_moments_updated.emit(_key_moments_cache)


## Calculate significance score for a decision (0.0 ~ 1.0)
func _calculate_significance(decision: Dictionary) -> float:
	var score = 0.0

	# High CA gain = significant
	if decision.outcome.has("ca_gain"):
		var ca_gain = decision.outcome.ca_gain
		if ca_gain >= 5:
			score += 0.8
		elif ca_gain >= 3:
			score += 0.5
		elif ca_gain >= 2:
			score += 0.3

	# Risky decisions = significant
	var condition = decision.context.get("condition", 100)
	if condition < 50 and _is_intense_training(decision.choice):
		score += 0.4

	# Injuries = significant
	if decision.outcome.has("injured") and decision.outcome.injured:
		score += 0.6

	# Major events = significant
	if decision.type == "event":
		score += 0.3

	return min(score, 1.0)


## Analyze which attributes were trained most
func _analyze_training_focus(training_decisions: Array) -> Array:
	# TODO: In Phase 2, integrate with TrainingManager to track which attributes
	# For now, return placeholder
	return ["Technical", "Physical"]


## Calculate balance score (how evenly distributed is training)
func _calculate_balance_score(training_decisions: Array) -> float:
	# TODO: In Phase 2, calculate from attribute distribution
	# For now, return 0.5 (balanced)
	return 0.5


## Get default patterns (when no decisions exist)
func _get_default_patterns() -> Dictionary:
	return {
		"training_tendency": "Balanced",
		"intensity_ratio": 0.5,
		"risk_taking": "Balanced",
		"risk_ratio": 0.0,
		"focus_areas": [],
		"balance_score": 0.5,
		"total_decisions": 0,
		"total_training": 0
	}
