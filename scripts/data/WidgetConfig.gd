extends Resource
class_name WidgetConfig
## Widget configuration data class
## Phase 13: Extended Features - Dashboard System

## Widget identification
@export var widget_id: String = ""
@export var widget_type: String = ""  # "chart", "stats", "quick_action"

## Layout properties
@export var grid_position: Vector2i = Vector2i(0, 0)  # Grid coordinates (x, y)
@export var grid_size: Vector2i = Vector2i(2, 2)  # Grid size (width, height)

## Chart-specific properties (if widget_type == "chart")
@export var chart_type: String = ""  # "line", "bar", "hexagon"
@export var data_source: String = ""  # "training_trends", "match_performance", "attribute_growth"

## Stats-specific properties (if widget_type == "stats")
@export var stats_type: String = ""  # "training", "match", "progress"
@export var display_mode: String = "compact"  # "compact", "detailed"

## Quick action properties (if widget_type == "quick_action")
@export var action_type: String = ""  # "filter", "export", "compare"
@export var button_text: String = ""

## Visual properties
@export var title: String = ""
@export var show_title: bool = true
@export var background_color: Color = Color(0.2, 0.3, 0.4, 0.9)
@export var border_color: Color = Color(0.4, 0.6, 0.8, 1.0)

## Animation properties
@export var animate_on_load: bool = true
@export var animation_type: String = "fade_in"  # "fade_in", "scale_in", "slide_in"

## Metadata
@export var created_at: String = ""
@export var last_modified: String = ""


## Create a duplicate of this config
func duplicate_config() -> WidgetConfig:
	var dup = WidgetConfig.new()
	dup.widget_id = widget_id
	dup.widget_type = widget_type
	dup.grid_position = grid_position
	dup.grid_size = grid_size
	dup.chart_type = chart_type
	dup.data_source = data_source
	dup.stats_type = stats_type
	dup.display_mode = display_mode
	dup.action_type = action_type
	dup.button_text = button_text
	dup.title = title
	dup.show_title = show_title
	dup.background_color = background_color
	dup.border_color = border_color
	dup.animate_on_load = animate_on_load
	dup.animation_type = animation_type
	dup.created_at = created_at
	dup.last_modified = last_modified
	return dup


## Convert to dictionary for saving
func to_dict() -> Dictionary:
	return {
		"widget_id": widget_id,
		"widget_type": widget_type,
		"grid_position": {"x": grid_position.x, "y": grid_position.y},
		"grid_size": {"w": grid_size.x, "h": grid_size.y},
		"chart_type": chart_type,
		"data_source": data_source,
		"stats_type": stats_type,
		"display_mode": display_mode,
		"action_type": action_type,
		"button_text": button_text,
		"title": title,
		"show_title": show_title,
		"background_color": background_color.to_html(),
		"border_color": border_color.to_html(),
		"animate_on_load": animate_on_load,
		"animation_type": animation_type,
		"created_at": created_at,
		"last_modified": last_modified
	}


## Create from dictionary (loading)
static func from_dict(data: Dictionary) -> WidgetConfig:
	var config = WidgetConfig.new()
	config.widget_id = data.get("widget_id", "")
	config.widget_type = data.get("widget_type", "")

	var pos = data.get("grid_position", {"x": 0, "y": 0})
	config.grid_position = Vector2i(pos.get("x", 0), pos.get("y", 0))

	var size = data.get("grid_size", {"w": 2, "h": 2})
	config.grid_size = Vector2i(size.get("w", 2), size.get("h", 2))

	config.chart_type = data.get("chart_type", "")
	config.data_source = data.get("data_source", "")
	config.stats_type = data.get("stats_type", "")
	config.display_mode = data.get("display_mode", "compact")
	config.action_type = data.get("action_type", "")
	config.button_text = data.get("button_text", "")
	config.title = data.get("title", "")
	config.show_title = data.get("show_title", true)

	var bg_color = data.get("background_color", "#33445580")
	config.background_color = Color.from_string(bg_color, Color(0.2, 0.3, 0.4, 0.9))

	var bd_color = data.get("border_color", "#6699ccff")
	config.border_color = Color.from_string(bd_color, Color(0.4, 0.6, 0.8, 1.0))

	config.animate_on_load = data.get("animate_on_load", true)
	config.animation_type = data.get("animation_type", "fade_in")
	config.created_at = data.get("created_at", "")
	config.last_modified = data.get("last_modified", "")

	return config


## Generate unique widget ID
static func generate_id() -> String:
	return "widget_%d_%d" % [Time.get_ticks_msec(), randi()]


## Validate configuration
func is_valid() -> bool:
	if widget_id == "":
		return false
	if widget_type == "":
		return false
	if grid_size.x <= 0 or grid_size.y <= 0:
		return false

	# Type-specific validation
	match widget_type:
		"chart":
			if chart_type == "" or data_source == "":
				return false
		"stats":
			if stats_type == "":
				return false
		"quick_action":
			if action_type == "":
				return false

	return true
