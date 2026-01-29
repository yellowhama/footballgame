## PipelinePresets - Configuration presets for match pipeline
## Part of Uber Realtime Architecture implementation
##
## Provides predefined configurations for different use cases:
## - SMOOTH: Best visual quality, higher CPU usage
## - BALANCED: Good balance of quality and performance (default)
## - PERFORMANCE: Maximum performance, may sacrifice some smoothness
##
## Reference: docs/specs/fix_2601/0113/UBER_REALTIME_ARCHITECTURE.md
##
## Usage:
##   var config = PipelinePresets.get_config(PipelinePresets.Preset.BALANCED)
##   delta_filter.configure(config.delta_filter)
##   aoi_selector.configure(config.aoi)
##
class_name PipelinePresets
extends RefCounted

enum Preset {
	SMOOTH,       # Best visual quality
	BALANCED,     # Default - good balance
	PERFORMANCE   # Maximum performance
}

## Get configuration for a preset
static func get_config(preset: Preset) -> Dictionary:
	match preset:
		Preset.SMOOTH:
			return {
				"name": "smooth",
				"description": "Best visual quality, higher CPU usage",
				"delta_filter": {
					"enabled": true,
					"ball_threshold": 0.2,      # 20cm - very sensitive
					"player_threshold": 0.2,    # 20cm
					"min_changed_players": 1    # Any player movement triggers
				},
				"aoi": {
					"mode": "FULL",             # Update all players every frame
					"radius": 100.0             # Effectively disabled
				},
				"buffer_size": 8,               # More frames for smoother interpolation
				"interpolation_delay_ms": 150,  # Larger buffer = more latency but smoother
				"dead_reckoning_max_ms": 300    # Shorter prediction window
			}

		Preset.BALANCED:
			return {
				"name": "balanced",
				"description": "Good balance of quality and performance",
				"delta_filter": {
					"enabled": true,
					"ball_threshold": 0.35,     # 35cm
					"player_threshold": 0.35,   # 35cm
					"min_changed_players": 2    # Need 2+ players changed
				},
				"aoi": {
					"mode": "BALL_CENTRIC",
					"radius": 20.0,             # 20m from ball = high priority
					"tier_0_radius": 10.0,
					"tier_1_radius": 20.0,
					"tier_2_radius": 30.0
				},
				"buffer_size": 4,               # Standard ring buffer
				"interpolation_delay_ms": 100,  # 100ms delay
				"dead_reckoning_max_ms": 500    # Up to 500ms prediction
			}

		Preset.PERFORMANCE:
			return {
				"name": "performance",
				"description": "Maximum performance, may sacrifice smoothness",
				"delta_filter": {
					"enabled": true,
					"ball_threshold": 0.6,      # 60cm - less sensitive
					"player_threshold": 0.6,    # 60cm
					"min_changed_players": 3    # Need 3+ players changed
				},
				"aoi": {
					"mode": "BALL_CENTRIC",
					"radius": 15.0,             # Smaller high-priority zone
					"tier_0_radius": 8.0,
					"tier_1_radius": 15.0,
					"tier_2_radius": 25.0
				},
				"buffer_size": 4,
				"interpolation_delay_ms": 50,   # Minimal delay
				"dead_reckoning_max_ms": 500
			}

	# Fallback to balanced
	return get_config(Preset.BALANCED)


## Get preset by name string
static func get_preset_by_name(name: String) -> Preset:
	match name.to_lower():
		"smooth", "quality", "high":
			return Preset.SMOOTH
		"balanced", "default", "medium":
			return Preset.BALANCED
		"performance", "fast", "low":
			return Preset.PERFORMANCE
	return Preset.BALANCED


## Get all preset names
static func get_preset_names() -> Array:
	return ["smooth", "balanced", "performance"]


## Get description for a preset
static func get_description(preset: Preset) -> String:
	var config = get_config(preset)
	return config.get("description", "")


## Apply preset to pipeline components
## Returns true if all components were configured successfully
static func apply_preset(
	preset: Preset,
	delta_filter,  # DeltaFilter instance
	aoi_selector = null,  # AOISelector instance (optional)
	pipeline = null  # UnifiedFramePipeline instance (optional)
) -> bool:
	var config = get_config(preset)
	var success = true

	# Configure DeltaFilter
	if delta_filter and delta_filter.has_method("configure"):
		delta_filter.configure(config.get("delta_filter", {}))
	else:
		success = false

	# Configure AOISelector (optional)
	if aoi_selector and aoi_selector.has_method("configure"):
		aoi_selector.configure(config.get("aoi", {}))

	# Configure pipeline settings (optional)
	if pipeline:
		if pipeline.has_method("set") or "enable_delta_filter" in pipeline:
			pipeline.enable_delta_filter = config.get("delta_filter", {}).get("enabled", true)

	if OS.is_debug_build():
		print("[PipelinePresets] Applied preset: %s" % config.get("name", "unknown"))

	return success


## Get recommended preset based on device capabilities
## (Simple heuristic - can be expanded with actual performance detection)
static func get_recommended_preset() -> Preset:
	# Check if running on mobile
	var os_name = OS.get_name()
	if os_name in ["Android", "iOS"]:
		return Preset.PERFORMANCE

	# Check processor count as simple heuristic
	var cpu_count = OS.get_processor_count()
	if cpu_count >= 8:
		return Preset.SMOOTH
	elif cpu_count >= 4:
		return Preset.BALANCED
	else:
		return Preset.PERFORMANCE
