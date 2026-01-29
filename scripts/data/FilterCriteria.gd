extends Resource
class_name FilterCriteria
## Filter criteria data class for history records
## Phase 13: Extended Features - Search/Filter System

## Search text (applied to all text fields)
@export var search_text: String = ""

## Match type filters
@export var include_friendly: bool = true
@export var include_league: bool = true
@export var include_cup: bool = true
@export var include_training: bool = true

## Result filters
@export var include_win: bool = true
@export var include_draw: bool = true
@export var include_loss: bool = true

## Date range filters
@export var date_from: String = ""  # Format: YYYY-MM-DD
@export var date_to: String = ""  # Format: YYYY-MM-DD

## Attribute filters (for training records)
@export var min_attribute_gain: float = 0.0
@export var max_attribute_gain: float = 100.0

## Sort options
enum SortField { DATE, RESULT, SCORE_DIFF, ATTRIBUTE_GAIN }

enum SortOrder { ASCENDING, DESCENDING }

@export var sort_field: SortField = SortField.DATE
@export var sort_order: SortOrder = SortOrder.DESCENDING


## Reset all filters to default (show all)
func reset():
	search_text = ""
	include_friendly = true
	include_league = true
	include_cup = true
	include_training = true
	include_win = true
	include_draw = true
	include_loss = true
	date_from = ""
	date_to = ""
	min_attribute_gain = 0.0
	max_attribute_gain = 100.0
	sort_field = SortField.DATE
	sort_order = SortOrder.DESCENDING


## Check if any active filters (excluding sort)
func has_active_filters() -> bool:
	if search_text != "":
		return true
	if not (include_friendly and include_league and include_cup and include_training):
		return true
	if not (include_win and include_draw and include_loss):
		return true
	if date_from != "" or date_to != "":
		return true
	if min_attribute_gain > 0.0 or max_attribute_gain < 100.0:
		return true
	return false


## Create a duplicate of this filter criteria
func duplicate_criteria():
	var dup = get_script().new()
	dup.search_text = search_text
	dup.include_friendly = include_friendly
	dup.include_league = include_league
	dup.include_cup = include_cup
	dup.include_training = include_training
	dup.include_win = include_win
	dup.include_draw = include_draw
	dup.include_loss = include_loss
	dup.date_from = date_from
	dup.date_to = date_to
	dup.min_attribute_gain = min_attribute_gain
	dup.max_attribute_gain = max_attribute_gain
	dup.sort_field = sort_field
	dup.sort_order = sort_order
	return dup
