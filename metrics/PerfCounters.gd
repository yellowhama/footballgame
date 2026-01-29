# Lightweight counters to attach to any scene for live profiling
extends Node
class_name PerfCounters

var avg_dt := 0.0
var smoothed_fps := 0.0
var frame_count := 0
var _alpha := 0.1

# Memory tracking (optional)
var peak_memory_mb := 0.0
var current_memory_mb := 0.0


func _ready() -> void:
	print("[PerfCounters] Performance monitoring started")


func _process(delta: float) -> void:
	# Exponential moving average for smooth FPS
	avg_dt = lerp(avg_dt, delta, _alpha)
	smoothed_fps = 1.0 / max(0.000001, avg_dt)
	frame_count += 1

	# Memory tracking (every 60 frames to reduce overhead)
	if frame_count % 60 == 0:
		_update_memory_stats()


func _update_memory_stats() -> void:
	# Godot 4.x: MEMORY_DYNAMIC 제거됨, MEMORY_STATIC_MAX 사용
	var static_memory = Performance.get_monitor(Performance.MEMORY_STATIC)
	var static_memory_max = Performance.get_monitor(Performance.MEMORY_STATIC_MAX)

	current_memory_mb = static_memory / (1024.0 * 1024.0)
	peak_memory_mb = max(peak_memory_mb, static_memory_max / (1024.0 * 1024.0))


func stats() -> Dictionary:
	return {
		"avg_dt": avg_dt,
		"fps": smoothed_fps,
		"frame_count": frame_count,
		"memory_mb": current_memory_mb,
		"peak_memory_mb": peak_memory_mb
	}


func get_fps() -> float:
	return smoothed_fps


func get_frametime_ms() -> float:
	return avg_dt * 1000.0


func reset() -> void:
	avg_dt = 0.0
	smoothed_fps = 0.0
	frame_count = 0
	peak_memory_mb = 0.0
	current_memory_mb = 0.0
