# Central policy management for Rust Bridge
extends RefCounted

# Feature flags
const USE_RUST_DEFAULT := true
const RUST_ENABLED_FILE := "user://rust_enabled.flag"

# Budget constants
const DEFAULT_WALL_MS := 50
const MAX_WALL_MS := 200
const MIN_WALL_MS := 10

# Retry policy
const RETRIES := 1
const RETRY_BACKOFF_MS := 25

# Health check
var _rust_healthy := true
var _last_health_check := 0
const HEALTH_CHECK_INTERVAL_MS := 30000  # 30초


static func should_use_rust() -> bool:
	# Check feature flag file
	if FileAccess.file_exists(RUST_ENABLED_FILE):
		var file = FileAccess.open(RUST_ENABLED_FILE, FileAccess.READ)
		if file:
			var enabled = file.get_as_text().strip_edges() == "true"
			file.close()
			return enabled

	# Default policy
	return USE_RUST_DEFAULT and ClassDB.class_exists("FootballMatchSimulator")


static func get_default_budget_ms() -> int:
	return DEFAULT_WALL_MS


static func clamp_wall(wall_ms: int) -> int:
	return clampi(wall_ms, MIN_WALL_MS, MAX_WALL_MS)


static func get_retry_count() -> int:
	return RETRIES


static func get_retry_backoff_ms() -> int:
	return RETRY_BACKOFF_MS


static func should_health_check(last_check_time: int) -> bool:
	var current_time = Time.get_ticks_msec()
	return (current_time - last_check_time) > HEALTH_CHECK_INTERVAL_MS


# Toggle Rust usage
static func set_rust_enabled(enabled: bool) -> void:
	var file = FileAccess.open(RUST_ENABLED_FILE, FileAccess.WRITE)
	if file:
		file.store_string("true" if enabled else "false")
		file.close()
		print("[Policy] Rust enabled: ", enabled)
	else:
		push_warning("[Policy] Failed to write rust_enabled flag")
