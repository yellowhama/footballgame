extends Node
## Filter service for history records
## Phase 13: Extended Features - Search/Filter System
## Autoload singleton: FilterService
# Preload to avoid autoload order issues with class_name
const _FilterCriteria = preload("res://scripts/data/FilterCriteria.gd")

signal filter_applied(filtered_count: int, total_count: int)


## Apply filters to match history records
func filter_match_records(records: Array, criteria: _FilterCriteria) -> Array:
	if not criteria:
		return records

	var filtered := []

	for record in records:
		if _match_passes_filter(record, criteria):
			filtered.append(record)

	# Apply sorting
	filtered = _sort_records(filtered, criteria.sort_field, criteria.sort_order)

	filter_applied.emit(filtered.size(), records.size())
	return filtered


## Apply filters to training history records
func filter_training_records(records: Array, criteria: _FilterCriteria) -> Array:
	if not criteria:
		return records

	var filtered := []

	for record in records:
		if _training_passes_filter(record, criteria):
			filtered.append(record)

	# Apply sorting
	filtered = _sort_records(filtered, criteria.sort_field, criteria.sort_order)

	filter_applied.emit(filtered.size(), records.size())
	return filtered


## Check if a match record passes filter criteria
func _match_passes_filter(record: Dictionary, criteria: _FilterCriteria) -> bool:
	# Match type filter
	var match_type = record.get("match_type", "friendly")
	match match_type:
		"friendly":
			if not criteria.include_friendly:
				return false
		"league":
			if not criteria.include_league:
				return false
		"cup":
			if not criteria.include_cup:
				return false

	# Result filter
	var result = record.get("result", "draw")
	match result:
		"win":
			if not criteria.include_win:
				return false
		"draw":
			if not criteria.include_draw:
				return false
		"loss":
			if not criteria.include_loss:
				return false

	# Date range filter
	var record_date = record.get("date", "")
	if criteria.date_from != "" and record_date < criteria.date_from:
		return false
	if criteria.date_to != "" and record_date > criteria.date_to:
		return false

	# Search text filter (check opponent, score, date)
	if criteria.search_text != "":
		var search_lower = criteria.search_text.to_lower()
		var opponent = record.get("opponent", "").to_lower()
		var score = str(record.get("score_for", 0)) + "-" + str(record.get("score_against", 0))
		var searchable = opponent + " " + score + " " + record_date

		if not searchable.contains(search_lower):
			return false

	return true


## Check if a training record passes filter criteria
func _training_passes_filter(record: Dictionary, criteria: _FilterCriteria) -> bool:
	# Type filter
	if not criteria.include_training:
		return false

	# Date range filter
	var record_date = record.get("date", "")
	if criteria.date_from != "" and record_date < criteria.date_from:
		return false
	if criteria.date_to != "" and record_date > criteria.date_to:
		return false

	# Attribute gain filter
	var total_gain = 0.0
	var attributes = record.get("attributes", {})
	for attr_name in attributes:
		var gain = attributes[attr_name].get("gain", 0.0)
		total_gain += abs(gain)

	if total_gain < criteria.min_attribute_gain:
		return false
	if total_gain > criteria.max_attribute_gain:
		return false

	# Search text filter (check training type, attributes)
	if criteria.search_text != "":
		var search_lower = criteria.search_text.to_lower()
		var training_type = record.get("training_type", "").to_lower()
		var attr_names = " ".join(attributes.keys()).to_lower()
		var searchable = training_type + " " + attr_names + " " + record_date

		if not searchable.contains(search_lower):
			return false

	return true


## Sort records by specified field and order
func _sort_records(records: Array, field: _FilterCriteria.SortField, order: _FilterCriteria.SortOrder) -> Array:
	var sorted = records.duplicate()

	match field:
		_FilterCriteria.SortField.DATE:
			sorted.sort_custom(
				func(a, b):
					var date_a = a.get("date", "")
					var date_b = b.get("date", "")
					if order == _FilterCriteria.SortOrder.ASCENDING:
						return date_a < date_b
					else:
						return date_a > date_b
			)

		_FilterCriteria.SortField.RESULT:
			sorted.sort_custom(
				func(a, b):
					var result_a = a.get("result", "draw")
					var result_b = b.get("result", "draw")
					var score_a = _result_to_score(result_a)
					var score_b = _result_to_score(result_b)
					if order == _FilterCriteria.SortOrder.ASCENDING:
						return score_a < score_b
					else:
						return score_a > score_b
			)

		_FilterCriteria.SortField.SCORE_DIFF:
			sorted.sort_custom(
				func(a, b):
					var diff_a = a.get("score_for", 0) - a.get("score_against", 0)
					var diff_b = b.get("score_for", 0) - b.get("score_against", 0)
					if order == _FilterCriteria.SortOrder.ASCENDING:
						return diff_a < diff_b
					else:
						return diff_a > diff_b
			)

		_FilterCriteria.SortField.ATTRIBUTE_GAIN:
			sorted.sort_custom(
				func(a, b):
					var gain_a = _calculate_total_gain(a)
					var gain_b = _calculate_total_gain(b)
					if order == _FilterCriteria.SortOrder.ASCENDING:
						return gain_a < gain_b
					else:
						return gain_a > gain_b
			)

	return sorted


## Convert result string to numeric score for sorting
func _result_to_score(result: String) -> int:
	match result:
		"win":
			return 3
		"draw":
			return 1
		"loss":
			return 0
		_:
			return 1


## Calculate total attribute gain from a record
func _calculate_total_gain(record: Dictionary) -> float:
	var total = 0.0
	var attributes = record.get("attributes", {})
	for attr_name in attributes:
		var gain = attributes[attr_name].get("gain", 0.0)
		total += abs(gain)
	return total
