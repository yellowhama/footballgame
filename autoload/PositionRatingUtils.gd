extends Node

## PositionRatingUtils: Shared utilities for FM 2023 position rating system
## Autoload singleton for position suitability checks and visualization
##
## FM 2023 Position Rating Scale (0-20):
## - 15-20: Natural position (no penalty) - Green
## - 11-14: Good position (15% penalty) - Yellow-green
## - 6-10: Adequate position (40% penalty) - Orange
## - 1-5: Very poor position (70% penalty) - Red
## - 0: Cannot play (100% penalty) - Dark gray
##
## Usage:
##   var color = PositionRatingUtils.get_position_rating_color(rating)
##   var icon = PositionRatingUtils.get_warning_icon(rating)
##   var tooltip = PositionRatingUtils.get_rating_tooltip(rating)

# Position rating color scheme (FM 2023 scale 0-20)
const COLOR_NATURAL = Color(0.0, 0.8, 0.0)  # 15-20: Green (natural position)
const COLOR_GOOD = Color(0.6, 0.8, 0.0)  # 11-14: Yellow-green (good)
const COLOR_ADEQUATE = Color(1.0, 0.6, 0.0)  # 6-10: Orange (adequate)
const COLOR_POOR = Color(0.8, 0.0, 0.0)  # 1-5: Red (very poor)
const COLOR_CANNOT = Color(0.2, 0.2, 0.2)  # 0: Dark gray (cannot play)


## Get color for position rating (FM 2023 scale 0-20)
## @param rating: Position rating value (0-20)
## @return Color corresponding to rating tier
static func get_position_rating_color(rating: int) -> Color:
	if rating >= 15:
		return COLOR_NATURAL
	elif rating >= 11:
		return COLOR_GOOD
	elif rating >= 6:
		return COLOR_ADEQUATE
	elif rating >= 1:
		return COLOR_POOR
	else:
		return COLOR_CANNOT


## Get warning level from rating
## @param rating: Position rating value (0-20)
## @return Warning level string: "none", "minor", "moderate", "major", "critical"
static func get_warning_level(rating: int) -> String:
	if rating >= 15:
		return "none"  # ✓ Natural position
	elif rating >= 11:
		return "minor"  # ○ Good
	elif rating >= 6:
		return "moderate"  # △ Adequate
	elif rating >= 1:
		return "major"  # ✗ Very poor
	else:
		return "critical"  # ⛔ Cannot play


## Get warning icon for rating
## @param rating: Position rating value (0-20)
## @return Icon string: "✓", "○", "△", "✗", "⛔", or ""
static func get_warning_icon(rating: int) -> String:
	match get_warning_level(rating):
		"none":
			return "✓"
		"minor":
			return "○"
		"moderate":
			return "△"
		"major":
			return "✗"
		"critical":
			return "⛔"
		_:
			return ""


## Calculate penalty percentage from rating
## @param rating: Position rating value (0-20)
## @return Penalty percentage (0-100)
static func calculate_penalty_percent(rating: int) -> int:
	if rating >= 15:
		return 0
	elif rating >= 11:
		return 15
	elif rating >= 6:
		return 40
	elif rating >= 1:
		return 70
	else:
		return 100


## Get tooltip text explaining rating and penalty
## @param rating: Position rating value (0-20)
## @return Tooltip text string
static func get_rating_tooltip(rating: int) -> String:
	if rating >= 15:
		return "Natural position (no penalty)"
	elif rating >= 11:
		return "Good position (15% penalty)"
	elif rating >= 6:
		return "Adequate position (40% penalty)"
	elif rating >= 1:
		return "Very poor position (70% penalty)"
	else:
		return "Cannot play this position (100% penalty)"


## Check if player is suitable for position
## @param rating: Position rating value (0-20)
## @return True if rating >= 11 (good or better)
static func is_suitable(rating: int) -> bool:
	return rating >= 11


## Get rating tier name (for UI display)
## @param rating: Position rating value (0-20)
## @return Tier name string
static func get_rating_tier_name(rating: int) -> String:
	if rating >= 15:
		return "Natural"
	elif rating >= 11:
		return "Good"
	elif rating >= 6:
		return "Adequate"
	elif rating >= 1:
		return "Poor"
	else:
		return "Cannot Play"
