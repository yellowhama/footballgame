# Autoload as BootTimer to measure startup. Logs in ms.
extends Node

var _t0 := Time.get_ticks_msec()
var _autoload_times := {}
var _total_boot_time := 0


func _ready():
	var ms := Time.get_ticks_msec() - _t0
	_total_boot_time = ms
	print("[BOOT][Autoload] BootTimer ready in ", ms, "ms")

	# Log other autoloads timing (if they call register_autoload)
	call_deferred("_log_summary")


func register_autoload(autoload_name: String, start_time: int) -> void:
	"""Call this from other autoloads to track their boot time"""
	var end_time = Time.get_ticks_msec()
	var duration = end_time - start_time
	_autoload_times[autoload_name] = duration
	print("[BOOT][Autoload] ", autoload_name, " init: ", duration, "ms")


func get_boot_time() -> int:
	return _total_boot_time


func get_autoload_times() -> Dictionary:
	return _autoload_times.duplicate()


func _log_summary() -> void:
	if _autoload_times.size() > 0:
		print("\n=== BOOT TIME SUMMARY ===")
		print("Total boot time: ", _total_boot_time, "ms")
		print("Autoload breakdown:")

		var total_tracked = 0
		for autoload_name in _autoload_times:
			var time = _autoload_times[autoload_name]
			total_tracked += time
			print("  ", autoload_name, ": ", time, "ms")

		if total_tracked > 0:
			print("Tracked autoloads: ", total_tracked, "ms")
			print("Untracked time: ", max(0, _total_boot_time - total_tracked), "ms")

		print("========================\n")


# Helper function for performance-critical sections
func start_timer() -> int:
	return Time.get_ticks_msec()


func end_timer(start_time: int, label: String = "") -> int:
	var duration = Time.get_ticks_msec() - start_time
	if label != "":
		print("[PERF] ", label, ": ", duration, "ms")
	return duration
