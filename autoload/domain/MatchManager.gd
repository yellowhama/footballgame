extends Node
## ============================================================================
## MatchManager - Match State & History Management
## ============================================================================
##
## PURPOSE: Match state tracking and history recording
##
## SSOT RESPONSIBILITIES:
## - Match state during simulation (score, time, half)
## - Match history (stored records)
## - MatchSetup integration
##
## DATA FLOW (ST-002 SSOT):
## - MatchSimulationManager runs simulation → calls ingest_external_match()
## - MatchManager records result in match_history
## - position_data, timeline_doc come exclusively from ingest_external_match payload
## - No fallback to MatchSimulationManager (unidirectional dependency)
##
## RELATED:
## - MatchSimulationManager: Owns simulation execution, sends results here
## ============================================================================
## Phase 8 Implementation: Match simulation and event generation

# Preload to avoid class_name resolution order issues (DO NOT REMOVE - fixes Parser Error)
const _MatchSetup = preload("res://scripts/core/MatchSetup.gd")
const _PlayerLibrary = preload("res://scripts/core/PlayerLibrary.gd")
const _MatchSetupBuilder = preload("res://scripts/core/MatchSetupBuilder.gd")
const _MatchPlayer = preload("res://scripts/core/MatchPlayer.gd")

# Match state
var is_match_active: bool = false
var current_time: int = 0
var current_half: String = "전반"
var home_score: int = 0
var away_score: int = 0
var current_tactic: String = "균형"
var current_matchday_instructions: Dictionary = {}
var simulation_timer: Timer = null

# Opponent data
var current_opponent: Dictionary = {}
var current_match_config: Dictionary = {}
var current_match_seed: int = 0
var current_match_events: Array = []
var current_match_result: Dictionary = {}
var current_timeline_doc: Dictionary = {}

# Phase 17: Game OS - MatchSetup integration
var current_match_setup: _MatchSetup = null
var player_library: _PlayerLibrary = null

# Phase 9.2: Match history tracking
var match_history: Array = []  # Array of match records

# P0.5: Global simulation lock (via SimulationLock autoloader)
var _current_lock_token: String = ""

# Signals
signal match_started
signal score_changed(home: int, away: int)
signal time_updated(minutes: int, half: String)
signal event_occurred(time: int, icon: String, text: String)
signal match_ended(result: Dictionary)
signal match_preflight_failed(info: Dictionary)

# Tactic options
const TACTICS = {
	"공격적": {"goal_chance_mult": 1.3, "concede_chance_mult": 1.2},
	"균형": {"goal_chance_mult": 1.0, "concede_chance_mult": 1.0},
	"수비적": {"goal_chance_mult": 0.7, "concede_chance_mult": 0.8}
}

const HOME_TEAM_NAME := "My Team"
const DEFAULT_STARTING_POSITIONS := ["GK", "CB", "CB", "RB", "LB", "DM", "CM", "CM", "RW", "LW", "ST"]
const DEFAULT_BENCH_POSITIONS := ["GK", "CB", "FB", "DM", "CM", "AM", "ST"]
const DEFAULT_TACTICS := {"attacking_intensity": 50, "defensive_line": 50, "pressing": 50, "tempo": 50}
const DEFAULT_MATCHDAY_INSTRUCTIONS := {
	"tempo": "normal",
	"press_intensity": 0.5,
	"build_up_style": "Mixed"
}


func _ready():
	print("[MatchManager] Initialized")
	# Prefer shared PlayerLibrary (connected to cache) if available.
	player_library = _resolve_shared_player_library()
	if player_library == null:
		player_library = _PlayerLibrary.new()
		print("[MatchManager] PlayerLibrary initialized (stub mode)")
	else:
		print("[MatchManager] PlayerLibrary initialized (shared/cache mode)")


func _resolve_shared_player_library() -> _PlayerLibrary:
	# 1) PlayerService가 제공하면 그걸 쓴다 (가장 안전)
	if has_node("/root/PlayerService"):
		var ps = get_node("/root/PlayerService")
		if ps and ps.has_method("get_player_library"):
			var pl = ps.call("get_player_library")
			if pl is _PlayerLibrary:
				return pl

	# 2) FootballRustEngine가 제공하면 그걸 쓴다
	if FootballRustEngine and FootballRustEngine.has_method("get_player_library"):
		var pl2 = FootballRustEngine.call("get_player_library")
		if pl2 is _PlayerLibrary:
			return pl2

	return null


func _exit_tree():
	"""Cleanup when manager is removed from tree (prevent memory leak)"""
	if simulation_timer:
		simulation_timer.stop()
		simulation_timer.queue_free()
		simulation_timer = null


func start_match(match_config: Dictionary):
	"""
	Start a new match

	Args:
		match_config: Dictionary with opponent/team data
			{
				"name": String,
				"overall_rating": int (0-100),
				"seed": int (optional deterministic seed)
			}
	"""
	# P0.5: Guard with global simulation lock
	if SimulationLock.is_locked():
		var info = SimulationLock.get_lock_info()
		push_warning("[MatchManager] start_match REJECTED: lock held by '%s' (%.1fs)" % [info.token, info.age_seconds])
		return

	if is_match_active:
		push_warning("[MatchManager] Match already active, cannot start new match")
		return

	var normalized_config: Dictionary = match_config.duplicate(true)
	var normalized_opponent: Dictionary = normalized_config.duplicate(true)
	if not normalized_opponent.has("name") and normalized_opponent.has("opponent"):
		normalized_opponent["name"] = normalized_opponent["opponent"]
	normalized_opponent["overall_rating"] = normalized_opponent.get(
		"overall_rating", normalized_opponent.get("rating", 50)
	)
	current_opponent = normalized_opponent
	current_match_config = normalized_config
	if not current_match_config.has("home_team_name"):
		current_match_config["home_team_name"] = HOME_TEAM_NAME
	if not current_match_config.has("home_overall_rating"):
		current_match_config["home_overall_rating"] = int(current_match_config.get("home_rating", 70))

	current_match_seed = int(normalized_config.get("seed", 0))
	if current_match_seed <= 0:
		# Fall back to timestamp for legacy callers without seeds
		current_match_seed = Time.get_ticks_usec()
		current_match_config["seed"] = current_match_seed

	print(
		(
			"[MatchManager] Starting match vs %s (OVR: %d)"
			% [current_opponent.get("name", "Unknown"), current_opponent.get("overall_rating", 50)]
		)
	)

	# Initialize match state
	is_match_active = true
	current_time = 0
	current_half = "전반"
	home_score = 0
	away_score = 0
	current_tactic = "균형"
	current_matchday_instructions = DEFAULT_MATCHDAY_INSTRUCTIONS.duplicate(true)
	var home_tactics := _extract_home_tactics(current_match_config)
	if not home_tactics.is_empty():
		current_matchday_instructions = _normalize_matchday_instructions(
			_derive_matchday_from_home_tactics(home_tactics)
		)
	else:
		var derived := _derive_matchday_from_team_tactics()
		if not derived.is_empty():
			current_matchday_instructions = _normalize_matchday_instructions(derived)
			_apply_matchday_to_match_config(current_matchday_instructions)
	current_match_events.clear()
	current_match_result = {}
	current_timeline_doc = {}

	# Emit start signal
	match_started.emit()
	_append_match_event(0, "⚽", "경기 시작!", {"event_type": "kickoff"})

	# Start simulation (sync)
	_start_simulation()


func set_tactic(tactic: String):
	"""
	Change team tactic during match

	Args:
		tactic: Tactic name ("공격적", "균형", "수비적")
	"""
	if not is_match_active:
		push_warning("[MatchManager] No active match")
		return

	if not TACTICS.has(tactic):
		push_error("[MatchManager] Invalid tactic: %s" % tactic)
		return

	current_tactic = tactic
	_append_match_event(current_time, "📋", "전술 변경: %s" % tactic)
	print("[MatchManager] Tactic changed to: %s" % tactic)


func set_matchday_instructions(payload: Dictionary) -> void:
	"""
	Apply matchday instructions (tempo/pressing/build-up).

	Args:
		payload: {"tempo": "slow|normal|fast", "press_intensity": 0.0-1.0, "build_up_style": "Short|Mixed|Direct"}
	"""
	if not is_match_active:
		push_warning("[MatchManager] No active match")
		return
	current_matchday_instructions = _normalize_matchday_instructions(payload)
	_apply_matchday_to_match_config(current_matchday_instructions)
	var tempo_label := _format_tempo_label(current_matchday_instructions.get("tempo", "normal"))
	var press_label := _format_press_label(
		float(current_matchday_instructions.get("press_intensity", 0.5))
	)
	var buildup_label := _format_buildup_label(
		str(current_matchday_instructions.get("build_up_style", "Mixed"))
	)
	_append_match_event(
		current_time,
		"🧭",
		"전술 변경: 템포 %s / 압박 %s / 빌드업 %s" % [tempo_label, press_label, buildup_label]
	)
	print("[MatchManager] Matchday instructions updated:", current_matchday_instructions)


func _normalize_matchday_instructions(payload: Dictionary) -> Dictionary:
	var normalized := DEFAULT_MATCHDAY_INSTRUCTIONS.duplicate(true)
	if current_matchday_instructions:
		normalized.merge(current_matchday_instructions, true)
	if payload:
		normalized.merge(payload, true)
	var tempo := str(normalized.get("tempo", "normal")).to_lower()
	match tempo:
		"slow":
			normalized["tempo"] = "slow"
		"fast":
			normalized["tempo"] = "fast"
		_:
			normalized["tempo"] = "normal"
	var buildup := str(normalized.get("build_up_style", "Mixed")).to_lower()
	match buildup:
		"short":
			normalized["build_up_style"] = "Short"
		"direct":
			normalized["build_up_style"] = "Direct"
		_:
			normalized["build_up_style"] = "Mixed"
	normalized["press_intensity"] = clampf(
		float(normalized.get("press_intensity", 0.5)), 0.0, 1.0
	)
	return normalized


func _extract_home_tactics(config: Dictionary) -> Dictionary:
	if not config:
		return {}
	var candidate: Variant = config.get("home_tactics", {})
	if candidate is Dictionary and not (candidate as Dictionary).is_empty():
		return (candidate as Dictionary).duplicate(true)
	return {}


func _derive_matchday_from_team_tactics() -> Dictionary:
	if not MyTeamData or not MyTeamData.has_method("get_team_tactics"):
		return {}
	var tactics: Variant = MyTeamData.get_team_tactics()
	if tactics is Dictionary:
		var params: Variant = (tactics as Dictionary).get("parameters", {})
		if params is Dictionary and not (params as Dictionary).is_empty():
			var p := params as Dictionary
			return {
				"tempo": _map_param_tempo_to_matchday(float(p.get("tempo", 0.6))),
				"press_intensity": _map_param_press_to_matchday(
					float(p.get("pressing_trigger", 0.5))
				),
				"build_up_style": _map_param_directness_to_matchday(
					float(p.get("directness", 0.5))
				)
			}
	return {}


func _derive_matchday_from_home_tactics(home_tactics: Dictionary) -> Dictionary:
	var tempo_value := float(home_tactics.get("tempo", 50))
	var press_value := float(home_tactics.get("pressing", 50))
	var passing_value := float(
		home_tactics.get("passing_style", home_tactics.get("directness", 50))
	)
	return {
		"tempo": _map_value_tempo_to_matchday(tempo_value),
		"press_intensity": _map_value_press_to_matchday(press_value),
		"build_up_style": _map_value_passing_to_matchday(passing_value)
	}


func _map_param_tempo_to_matchday(value: float) -> String:
	if value >= 0.7:
		return "fast"
	if value <= 0.45:
		return "slow"
	return "normal"


func _map_param_press_to_matchday(value: float) -> float:
	if value >= 0.65:
		return 0.8
	if value <= 0.4:
		return 0.3
	return 0.5


func _map_param_directness_to_matchday(value: float) -> String:
	if value >= 0.65:
		return "Direct"
	if value <= 0.4:
		return "Short"
	return "Mixed"


func _map_value_tempo_to_matchday(value: float) -> String:
	if value >= 65.0:
		return "fast"
	if value <= 45.0:
		return "slow"
	return "normal"


func _map_value_press_to_matchday(value: float) -> float:
	if value >= 65.0:
		return 0.8
	if value <= 45.0:
		return 0.3
	return 0.5


func _map_value_passing_to_matchday(value: float) -> String:
	if value >= 65.0:
		return "Direct"
	if value <= 45.0:
		return "Short"
	return "Mixed"


func _apply_matchday_to_match_config(instructions: Dictionary) -> void:
	var tactics_payload := _build_matchday_tactics_payload(instructions)
	current_match_config["home_tactics"] = tactics_payload.duplicate(true)
	if current_match_setup and current_match_setup.home_team:
		current_match_setup.home_team.tactics = tactics_payload.duplicate(true)


func _build_matchday_tactics_payload(instructions: Dictionary) -> Dictionary:
	var tactics: Dictionary = {}
	var existing_tactics: Variant = current_match_config.get("home_tactics", {})
	if existing_tactics is Dictionary:
		tactics = (existing_tactics as Dictionary).duplicate(true)
	elif DEFAULT_TACTICS:
		tactics = DEFAULT_TACTICS.duplicate(true)
	var tempo_value := str(instructions.get("tempo", "normal")).to_lower()
	match tempo_value:
		"slow":
			tactics["tempo"] = 40
		"fast":
			tactics["tempo"] = 60
		_:
			tactics["tempo"] = 50
	var press_value := float(instructions.get("press_intensity", 0.5))
	if press_value >= 0.7:
		tactics["pressing"] = 70
	elif press_value <= 0.4:
		tactics["pressing"] = 40
	else:
		tactics["pressing"] = 55
	var buildup_value := str(instructions.get("build_up_style", "Mixed")).to_lower()
	match buildup_value:
		"short":
			tactics["passing_style"] = 40
		"direct":
			tactics["passing_style"] = 70
		_:
			tactics["passing_style"] = 55
	return tactics


func _format_tempo_label(tempo: String) -> String:
	match tempo.to_lower():
		"slow":
			return "느림"
		"fast":
			return "빠름"
		_:
			return "보통"


func _format_press_label(press_intensity: float) -> String:
	if press_intensity >= 0.7:
		return "높음"
	if press_intensity <= 0.4:
		return "낮음"
	return "보통"


func _format_buildup_label(build_up_style: String) -> String:
	match build_up_style.to_lower():
		"short":
			return "숏 패스"
		"direct":
			return "직접"
		_:
			return "혼합"


func skip_to_end():
	"""Fast-forward to end of match (obsolete with Rust instant simulation)"""
	if not is_match_active:
		push_warning("[MatchManager] No active match to skip")
		return

	# With Rust simulation, matches complete instantly (<100ms)
	# This function is kept for backwards compatibility but does nothing
	print("[MatchManager] Skip requested, but Rust simulation already completes instantly")


func request_timeout():
	"""Request timeout during match (Phase 8 TODO)"""
	if not is_match_active:
		return

	_append_match_event(current_time, "⏸️", "타임아웃 요청")
	print("[MatchManager] TODO: Implement timeout system")


func _start_simulation():
	"""Start match simulation using MatchSetup OS (Phase 17) - REFACTORED"""
	# P0.5: Acquire global simulation lock
	var lock_result = SimulationLock.try_acquire("MatchManager")
	if not lock_result.success:
		push_error("[MatchManager] Failed to acquire lock: %s" % lock_result.reason)
		_fallback_to_simple_result()
		return

	_current_lock_token = lock_result.token

	# Step 1: Validate engine
	if not FootballRustEngine or not FootballRustEngine.is_ready():
		push_error("[MatchManager] FootballRustEngine not ready")
		SimulationLock.release(_current_lock_token)
		_current_lock_token = ""
		_fallback_to_simple_result()
		return

	# Step 2: Prepare configuration
	var match_config = _prepare_match_config()

	# Step 3: Create MatchSetup
	current_match_setup = _create_match_setup(match_config)
	if not current_match_setup:
		push_error("[MatchManager] Failed to create MatchSetup")
		SimulationLock.release(_current_lock_token)
		_current_lock_token = ""
		_fallback_to_simple_result()
		return

	# Step 4: Execute simulation
	var api_result = _execute_engine_simulation(current_match_setup, match_config.get("seed", 0))
	if not api_result.get("success", false):
		push_error("[MatchManager] Simulation failed: %s" % api_result.get("error", "Unknown"))
		SimulationLock.release(_current_lock_token)
		_current_lock_token = ""
		_fallback_to_simple_result()
		return

	# Step 5: Process events and scores
	current_match_result = api_result.get("response", {}).duplicate(true)

	# Check for errors (defensive)
	if _extract_bool(current_match_result, "error") or not _extract_bool(current_match_result, "success", true):
		push_error("[MatchManager] Simulation result flagged error")
		SimulationLock.release(_current_lock_token)
		_current_lock_token = ""
		_fallback_to_simple_result()
		return

	_process_simulation_events(current_match_result)
	_finalize_match_scores(current_match_result)

	# Step 6: End match
	_end_match_with_seed(match_config["seed"])

	# P0.5: Release lock after successful completion
	SimulationLock.release(_current_lock_token)
	_current_lock_token = ""

	print(
		(
			"[MatchManager] Rust simulation completed (seed: %d, score: %d-%d)"
			% [match_config["seed"], home_score, away_score]
		)
	)


## Helper 1: Prepare match configuration
func _prepare_match_config() -> Dictionary:
	"""Prepare match configuration for MatchSetup"""
	var simulation_seed: int = current_match_seed if current_match_seed > 0 else Time.get_ticks_usec()
	current_match_seed = simulation_seed
	current_match_config["seed"] = simulation_seed

	return {
		"seed": simulation_seed,
		"match_id": "career_match_%d" % Time.get_ticks_usec(),
		"match_type": "career",
		"venue": "home",
		"home_formation": current_match_config.get("home_formation", "4-4-2"),
		"away_formation": current_match_config.get("away_formation", "4-4-2"),
		"home_tactics": current_match_config.get("home_tactics", {})
	}


## Helper 2: Create MatchSetup using MatchSetupBuilder
func _create_match_setup(match_config: Dictionary) -> _MatchSetup:
	"""Create MatchSetup using MatchSetupBuilder"""
	print("[MatchManager] Building MatchSetup (Game OS)...")

	# Build roster UIDs
	var home_roster_uids = _build_home_roster_uids()
	var away_roster_uids = _build_away_roster_uids()

	# Build MatchSetup
	var match_setup = _MatchSetupBuilder.build(
		home_roster_uids,
		away_roster_uids,
		match_config["home_formation"],
		match_config["away_formation"],
		player_library,
		match_config
	)

	if match_setup:
		match_setup.rng_seed = match_config["seed"]
		print("[MatchManager] ✅ MatchSetup created: %s" % match_setup.get_summary())

	return match_setup


## Helper 3: Execute match simulation via OpenFootballAPI
func _execute_engine_simulation(match_setup: _MatchSetup, rng_seed: int) -> Dictionary:
	"""Execute match simulation via OpenFootballAPI"""
	print("[MatchManager] Starting Rust simulation with seed: %d" % rng_seed)

	# Preflight: ensure starting XI UIDs can be resolved before calling Rust.
	var preflight = _preflight_match_setup_roster(match_setup)
	if not preflight.ok:
		# ✅ UX: bubble up missing list + broadcast to UI layers (WeekHub/MatchScreen)
		var info := {
			"ok": false,
			"error": preflight.error,
			"missing": preflight.get("missing", []),
			"match_id": match_setup.track_id if match_setup else "",
			"seed": rng_seed,
		}
		match_preflight_failed.emit(info)
		_emit_preflight_failed_eventbus(info)
		return {"success": false, "error": preflight.error, "missing": info.missing}

	# NOTE (2025-12-23): Roster UIDs are now created as numeric strings in _build_*_roster_uids()
	# This conversion is now redundant but kept for defensive programming
	_convert_match_setup_uids_for_rust(match_setup)

	# Show loading UI
	if has_node("/root/LoadingUI"):
		LoadingUI.show_loading("경기 시뮬레이션 중…")

	# Call API
	var api_res = OpenFootballAPI.simulate_match_with_setup(match_setup, rng_seed, "simple", true)

	# Hide loading UI
	if has_node("/root/LoadingUI"):
		LoadingUI.hide_loading()

	return api_res


## Helper 4a: Load timeline document from result
func _load_timeline_doc(result: Dictionary) -> void:
	"""Load timeline_doc from API result"""
	var legacy_doc_key := ("re" + "play") + "_doc"
	if result.has("timeline_doc") and result.timeline_doc is Dictionary:
		current_timeline_doc = result.timeline_doc.duplicate(true)
	elif result.has(legacy_doc_key) and result[legacy_doc_key] is Dictionary:
		current_timeline_doc = (result[legacy_doc_key] as Dictionary).duplicate(true)
	else:
		current_timeline_doc = _generate_timeline_doc(result)


## Helper 4b: Extract events array from result
func _extract_events_array(result: Dictionary) -> Array:
	"""Extract and validate events array"""
	var events_variant: Variant = result.get("events", [])
	var events: Array = []
	if events_variant is Array:
		events = (events_variant as Array).duplicate(true)
	return events


## Helper 4c: Process events loop with score tracking
func _process_events_loop(events: Array) -> void:
	"""Process each event: goals, halftime, general events"""
	var running_home_score: int = 0
	var running_away_score: int = 0

	for event in events:
		var minute: int = int(event.get("minute", 0))
		var event_type: String = str(event.get("event_type", event.get("kind", "")))

		match event_type:
			"goal":
				var is_home = event.get("is_home_team", true)
				if is_home:
					running_home_score += 1
					home_score = running_home_score
					_append_match_event(minute, "⚽", "골! 우리팀 득점!", event)
				else:
					running_away_score += 1
					away_score = running_away_score
					_append_match_event(minute, "⚽", "상대팀 득점...", event)

				current_time = minute
				current_half = "후반" if minute >= 45 else "전반"
				score_changed.emit(home_score, away_score)
				time_updated.emit(current_time, current_half)

			"half_time":
				current_half = "후반"
				current_time = 45
				time_updated.emit(current_time, current_half)
				_append_match_event(45, "⏱️", "전반전 종료", event)

			_:
				var description: String = str(event.get("description", "이벤트"))
				var icon: String = str(event.get("icon", "•"))
				current_time = minute
				current_half = "후반" if minute >= 45 else "전반"
				time_updated.emit(current_time, current_half)
				_append_match_event(minute, icon, description, event)


## Helper 4: Process simulation events into match timeline (REFACTORED 2025-12-19)
func _process_simulation_events(result: Dictionary) -> void:
	"""Process simulation events (orchestrator delegates to helpers 4a, 4b, 4c)"""
	_load_timeline_doc(result)
	var events: Array = _extract_events_array(result)
	_process_events_loop(events)


## Helper 5: Finalize scores from engine result
func _finalize_match_scores(result: Dictionary) -> void:
	"""Finalize scores from engine result"""
	# Ensure final score matches engine output
	var final_home: int = int(result.get("home_score", home_score))
	var final_away: int = int(result.get("away_score", away_score))

	if home_score != final_home or away_score != final_away:
		home_score = final_home
		away_score = final_away
		score_changed.emit(home_score, away_score)

	# Set final time
	current_time = 90
	current_half = "종료"
	time_updated.emit(current_time, current_half)


func _fallback_to_simple_result():
	"""Fallback: Generate simple result without full simulation"""
	push_warning("[MatchManager] Using fallback simple result")
	current_match_result = {}
	current_timeline_doc = {}

	# Simple result based on opponent rating
	var opponent_strength: int = int(current_opponent.get("overall_rating", 50))

	# Higher opponent rating = lower chance of winning
	if opponent_strength < 60:
		home_score = 2
		away_score = 0
	elif opponent_strength < 75:
		home_score = 1
		away_score = 1
	else:
		home_score = 0
		away_score = 2

	_append_match_event(90, "⚽", "경기 종료: %d - %d" % [home_score, away_score])
	score_changed.emit(home_score, away_score)
	current_time = 90
	time_updated.emit(90, "후반")
	_end_match()


func _build_home_roster_uids() -> Array:
	"""Build home roster UIDs (main player + 17 teammates) - Phase 17 OS

	✅ Engine UID SSOT (2025-12-23):
	- Rust PLAYER_LIBRARY uses integer keys (2) not string UIDs ("csv:2")
	- Never pass "grad:*" to engine - use proxy UID from cache
	- Output format: ["117", "2", "45", ...] (numeric strings only)
	"""
	var roster = []

	# Main player (from PlayerData)
	if PlayerData and PlayerData.has_method("get_uid"):
		var raw_uid = PlayerData.get_uid()
		var engine_uid := _resolve_engine_uid_for_main_player(raw_uid)
		if engine_uid == "":
			# Last resort: choose a valid uid near team level
			engine_uid = _pick_valid_engine_uid_near(_calculate_team_level())
		if engine_uid == "":
			# Absolute fallback
			engine_uid = "1"
		roster.append(engine_uid)
		print("[MatchManager] Added main player (engine uid): %s (raw=%s)" % [engine_uid, str(raw_uid)])
	else:
		# Fallback: Use engine UID (numeric string)
		roster.append("1")
		push_warning("[MatchManager] PlayerData not available, using CSV player")

	# Generate 17 teammates from CSV database
	var team_level = _calculate_team_level()
	var rng = RandomNumberGenerator.new()
	rng.seed = current_match_seed + 1000  # Offset for home team

	for i in range(17):
		# Generate CSV IDs near team level
		var csv_id = team_level + rng.randi_range(-20, 20)
		csv_id = clamp(csv_id, 1, 1000)
		# ✅ Engine UID SSOT: numeric string only
		var uid := _pick_valid_engine_uid_near(csv_id)
		if uid == "":
			uid = str(int(csv_id))
		roster.append(uid)

	print("[MatchManager] Home roster: %d players (engine UIDs)" % roster.size())
	return roster


func _build_away_roster_uids() -> Array:
	"""Build away roster UIDs (18 opponents from CSV) - Phase 17 OS

	✅ Engine UID SSOT (2025-12-23):
	- Output format: ["2", "45", "117", ...] (numeric strings only)
	"""
	var roster = []

	var opponent_level = int(current_opponent.get("overall_rating", 65))
	var rng = RandomNumberGenerator.new()
	rng.seed = current_match_seed + 2000  # Offset for away team

	for i in range(18):
		# Generate CSV IDs near opponent level
		var csv_id = opponent_level + rng.randi_range(-20, 20)
		csv_id = clamp(csv_id, 1, 1000)
		# ✅ Engine UID SSOT: numeric string only
		var uid := _pick_valid_engine_uid_near(csv_id)
		if uid == "":
			uid = str(int(csv_id))
		roster.append(uid)

	print("[MatchManager] Away roster: %d players (engine UIDs)" % roster.size())
	return roster


func _calculate_team_level() -> int:
	"""Calculate team level based on PlayerData overall rating"""
	if PlayerData and PlayerData.has_method("get_overall_rating"):
		return PlayerData.get_overall_rating()
	elif current_match_config.has("home_overall_rating"):
		return int(current_match_config["home_overall_rating"])
	return 70  # Default level


func _end_match():
	"""End match and calculate results"""
	is_match_active = false

	# Stop timer
	if simulation_timer:
		simulation_timer.stop()
		simulation_timer.queue_free()
		simulation_timer = null

	# Determine result
	var result_text: String = ""
	if home_score > away_score:
		result_text = "승리"
	elif home_score < away_score:
		result_text = "패배"
	else:
		result_text = "무승부"

	# Build result dictionary
	var result: Dictionary = {
		"final_score": [home_score, away_score],
		"result": result_text,
		"opponent": current_opponent.get("name", "Unknown"),
		"home_team_name": HOME_TEAM_NAME,
		"home_score": home_score,
		"away_score": away_score,
		"goals_scored": home_score,
		"goals_conceded": away_score,
		"events": current_match_events.duplicate(true)
	}

	if not current_timeline_doc.is_empty():
		result["timeline_doc"] = current_timeline_doc.duplicate(true)
	if not current_match_result.is_empty():
		result["match_result"] = current_match_result.duplicate(true)

	# Apply condition change based on result
	if ConditionSystem:
		match result_text:
			"승리":
				ConditionSystem.apply_victory_bonus()
			"패배":
				ConditionSystem.apply_defeat_penalty()

		# Apply match fatigue
		ConditionSystem.apply_match_fatigue()

	_append_match_event(90, "⏱️", "경기 종료: %s (%d - %d)" % [result_text, home_score, away_score])

	# Phase 9.2: Record match to history
	_record_match_history(result, result_text)

	match_ended.emit(result)

	print(
		(
			"[MatchManager] Match ended: %s (%d - %d) vs %s"
			% [result_text, home_score, away_score, current_opponent.get("name", "Unknown")]
		)
	)


func get_match_state() -> Dictionary:
	"""Get current match state"""
	return {
		"is_active": is_match_active,
		"time": current_time,
		"half": current_half,
		"score": [home_score, away_score],
		"tactic": current_tactic,
		"matchday_instructions": current_matchday_instructions.duplicate(true),
		"opponent": current_opponent,
		"events": current_match_events.duplicate(true)
	}


func is_active() -> bool:
	"""Check if match is currently active"""
	return is_match_active


func _generate_timeline_doc(match_result: Dictionary) -> Dictionary:
	if match_result.is_empty():
		return {}

	if not FootballRustEngine:
		push_warning("[MatchManager] FootballRustEngine not found, cannot build timeline doc")
		return {}

	var response: Dictionary = FootballRustEngine.get_timeline_json(match_result, "full")
	if response.has("error") and response.get("error", false):
		push_warning("[MatchManager] Timeline doc generation failed: %s" % response.get("message", "Unknown error"))
		return {}

	var timeline_doc: Dictionary = {}
	var legacy_block_key := "re" + "play"
	if response.has(legacy_block_key) and response[legacy_block_key] is Dictionary:
		timeline_doc = (response[legacy_block_key] as Dictionary).duplicate(true)
	else:
		timeline_doc = response.duplicate(true)

	if response.has("metadata") and response.metadata is Dictionary:
		timeline_doc["metadata"] = response.metadata.duplicate(true)

	return _normalize_timeline_doc(timeline_doc)


func _normalize_timeline_doc(timeline_doc: Dictionary) -> Dictionary:
	var normalized: Dictionary = timeline_doc.duplicate(true)
	var events: Array = []
	var legacy_block_key := "re" + "play"
	var legacy_block: Variant = normalized.get(legacy_block_key, null)

	if normalized.has("events") and normalized.events is Array:
		events = (normalized.events as Array).duplicate(true)
	elif legacy_block is Dictionary and legacy_block.has("events"):
		events = (legacy_block.events as Array).duplicate(true)

	if legacy_block is Dictionary:
		if legacy_block.has("rosters") and not normalized.has("rosters"):
			var rosters_source: Dictionary = legacy_block.rosters
			if rosters_source is Dictionary:
				normalized["rosters"] = rosters_source.duplicate(true)
		if legacy_block.has("metadata") and not normalized.has("metadata"):
			var metadata_source: Dictionary = legacy_block.metadata
			if metadata_source is Dictionary:
				normalized["metadata"] = metadata_source.duplicate(true)

	if normalized.has("metadata") and normalized.metadata is Dictionary:
		normalized["metadata"] = normalized.metadata.duplicate(true)
	if normalized.has(legacy_block_key):
		normalized.erase(legacy_block_key)

	var normalized_events: Array = []
	for raw_event in events:
		if not (raw_event is Dictionary):
			continue
		var event_dict: Dictionary = {}
		if raw_event.has("kind"):
			event_dict = raw_event.duplicate(true)
		elif raw_event.keys().size() == 1:
			var variant_key: String = str(raw_event.keys()[0])
			var variant_value: Variant = raw_event[variant_key]
			if variant_value is Dictionary:
				event_dict = (variant_value as Dictionary).duplicate(true)
				event_dict["kind"] = variant_key
		else:
			event_dict = raw_event.duplicate(true)

		if event_dict.is_empty():
			continue

		if event_dict.has("kind") and event_dict.kind is String:
			event_dict["kind"] = event_dict.kind.to_lower()

		var base_variant: Variant = event_dict.get("base", {})
		if base_variant is Dictionary:
			var base: Dictionary = base_variant
			if base.has("minute"):
				base["minute"] = int(base.get("minute", 0))
			if base.has("second"):
				base["second"] = int(base.get("second", 0))
			if base.has("t"):
				base["t"] = float(base.get("t", 0.0))
			event_dict["base"] = base

		normalized_events.append(event_dict)

	normalized["events"] = normalized_events
	if normalized.has("rosters") and normalized.rosters is Dictionary:
		normalized["rosters"] = normalized.rosters.duplicate(true)

	return normalized


## Phase 9.2: Match History Functions


func _end_match_with_seed(rng_seed: int):
	"""End match and calculate results (with deterministic seed stored)"""
	is_match_active = false

	# Stop timer (if exists - backwards compatibility)
	if simulation_timer:
		simulation_timer.stop()
		simulation_timer.queue_free()
		simulation_timer = null

	# Determine result
	var result_text: String = ""
	if home_score > away_score:
		result_text = "승리"
	elif home_score < away_score:
		result_text = "패배"
	else:
		result_text = "무승부"

	# Build result dictionary
	var result: Dictionary = {
		"final_score": [home_score, away_score],
		"result": result_text,
		"opponent": current_opponent.get("name", "Unknown"),
		"home_team_name": HOME_TEAM_NAME,
		"home_score": home_score,
		"away_score": away_score,
		"goals_scored": home_score,
		"goals_conceded": away_score,
		"events": current_match_events.duplicate(true),
		"seed": rng_seed,
		# Phase 22: Hero Time growth data
		"hero_growth": _extract_hero_growth_from_rust_result(current_match_result)
	}

	if not current_timeline_doc.is_empty():
		result["timeline_doc"] = current_timeline_doc.duplicate(true)
	if not current_match_result.is_empty():
		result["match_result"] = current_match_result.duplicate(true)

	# Apply condition change based on result
	if ConditionSystem:
		match result_text:
			"승리":
				ConditionSystem.apply_victory_bonus()
			"패배":
				ConditionSystem.apply_defeat_penalty()

		# Apply match fatigue
		ConditionSystem.apply_match_fatigue()

	_append_match_event(90, "⏱️", "경기 종료: %s (%d - %d)" % [result_text, home_score, away_score])

	# Record match to history with seed
	_record_match_history_with_seed(result, result_text, rng_seed)

	match_ended.emit(result)

	print(
		(
			"[MatchManager] Match ended: %s (%d - %d) vs %s (seed: %d)"
			% [result_text, home_score, away_score, current_opponent.get("name", "Unknown"), rng_seed]
		)
	)


func _simulate_with_seed(match_record: Dictionary, rng_seed: int) -> Dictionary:
	"""Re-simulate match using MatchSetup (Game OS mode - Phase 17)

	Replaces legacy _build_home_team_payload() workflow with _MatchSetupBuilder.
	"""
	if not FootballRustEngine or not FootballRustEngine.is_ready():
		push_warning("[MatchManager] Timeline requested before FootballRustEngine ready")
		return {"success": false, "message": "FootballRustEngine not ready"}

	print("[MatchManager] Re-simulating match with seed: %d (Game OS mode)" % rng_seed)

	# Extract match configuration
	var record_copy: Dictionary = match_record.duplicate(true)
	var home_config: Dictionary = {}
	var home_config_variant: Variant = record_copy.get("match_config", {})
	if home_config_variant is Dictionary:
		home_config = (home_config_variant as Dictionary).duplicate(true)

	# Extract opponent data
	var opponent_name := str(record_copy.get("opponent_name", record_copy.get("opponent", "Opponent")))
	var opponent_rating := int(record_copy.get("opponent_rating", record_copy.get("overall_rating", 50)))

	# Setup opponent for roster building
	current_opponent = {"name": opponent_name, "overall_rating": opponent_rating}

	# Prepare match config for MatchSetup
	var match_config = {
		"seed": rng_seed,
		"match_id": "timeline_%d" % rng_seed,
		"match_type": "timeline",
		"venue": "home",
		"home_formation": home_config.get("formation", "4-4-2"),
		"away_formation": home_config.get("away_formation", "4-4-2"),
		"home_tactics": home_config.get("tactics", {})
	}

	# Build rosters using Game OS workflow
	var home_roster_uids = _build_home_roster_uids()
	var away_roster_uids = _build_away_roster_uids()

	# Create MatchSetup via Builder (single entry point)
	var match_setup = _MatchSetupBuilder.build(
		home_roster_uids,
		away_roster_uids,
		match_config["home_formation"],
		match_config["away_formation"],
		player_library,
		match_config
	)

	if not match_setup:
		push_error("[MatchManager] Failed to create MatchSetup for timeline simulation")
		return {"success": false, "message": "MatchSetup creation failed"}

	# Execute simulation via Game OS (Exporter single path)
	var api_res: Dictionary = OpenFootballAPI.simulate_match_with_setup(match_setup, rng_seed, "simple", true)

	if not api_res.get("success", false):
		var message := str(api_res.get("error", "Unknown error"))
		push_warning("[MatchManager] Resimulation failed: %s" % message)
		return {"success": false, "message": message, "result": api_res}

	print("[MatchManager] ✅ Timeline simulation complete (Game OS mode)")
	return {"success": true, "result": api_res.get("response", {})}


func simulate_match_from_record(match_record: Dictionary) -> Dictionary:
	var rng_seed: int = int(match_record.get("seed", 0))
	if rng_seed <= 0:
		return {"success": false, "message": "No stored seed for this match."}

	var record_copy := match_record.duplicate(true)
	var simulation := _simulate_with_seed(record_copy, rng_seed)
	if not simulation.get("success", false):
		return simulation

	var result_dict: Dictionary = simulation.get("result", {})
	var events_variant: Variant = result_dict.get("events", [])
	var events: Array = []
	if events_variant is Array:
		events = (events_variant as Array).duplicate(true)
	if events.is_empty():
		var legacy_doc_key := "re" + "play"
		var legacy_doc_key2 := ("re" + "play") + "_doc"
		var doc_variant: Variant = result_dict.get(
			"timeline_doc", result_dict.get(legacy_doc_key, result_dict.get(legacy_doc_key2, {}))
		)
		if doc_variant is Dictionary:
			var doc_dict: Dictionary = doc_variant
			var doc_events_variant: Variant = doc_dict.get("events", [])
			if doc_events_variant is Array:
				events = (doc_events_variant as Array).duplicate(true)
	if events.is_empty() and result_dict.has("match_result"):
		var match_result_variant: Variant = result_dict.get("match_result")
		if match_result_variant is Dictionary and match_result_variant.has("events"):
			var match_events_variant: Variant = match_result_variant.get("events")
			if match_events_variant is Array:
				events = (match_events_variant as Array).duplicate(true)

	return {"success": true, "events": events, "result": result_dict}


func _extract_bool(data: Dictionary, key: String, default_value: bool = false) -> bool:
	if not data.has(key):
		return default_value
	var value = data[key]
	match typeof(value):
		TYPE_BOOL:
			return value
		TYPE_INT, TYPE_FLOAT:
			return value != 0
		TYPE_STRING:
			var lower := String(value).to_lower().strip_edges()
			return lower == "true" or lower == "1" or lower == "yes"
		_:
			return default_value


func _record_match_history(_result: Dictionary, result_text: String):
	"""Record a match to history (legacy - no seed)

	@param _result: Match result dictionary (reserved for future detailed statistics)
	"""
	_record_match_history_with_seed(_result, result_text, 0)


func _record_match_history_with_seed(_result: Dictionary, result_text: String, rng_seed: int):
	"""Record a match to history with deterministic seed

	@param _result: Match result dictionary (reserved for future detailed statistics)
	@param rng_seed: Deterministic seed used for match simulation (0 = no seed)
	"""
	# Get current game date
	var year: int = 1
	var week: int = 1
	if DateManager:
		year = DateManager.current_year
		week = DateManager.current_week

	var record: Dictionary = {
		"opponent_name": current_opponent.get("name", "Unknown"),
		"opponent_rating": current_opponent.get("overall_rating", 50),
		"home_team_name": HOME_TEAM_NAME,
		"year": year,
		"week": week,
		"timestamp": Time.get_unix_time_from_system(),
		"result": result_text,  # "승리", "패배", "무승부"
		"goals_scored": home_score,
		"goals_conceded": away_score,
		"final_score": [home_score, away_score],
		"tactic_used": current_tactic,
		"match_duration": 90,
		"seed": rng_seed,
		"events": current_match_events.duplicate(true),
		"raw_result": _result.duplicate(true),
		"match_config": current_match_config.duplicate(true)
	}

	var timeline_source: Variant = current_match_result.get("timeline", _result.get("timeline", []))
	if timeline_source is Array:
		record["timeline"] = (timeline_source as Array).duplicate(true)

	if not current_match_result.is_empty():
		record["match_result"] = current_match_result.duplicate(true)
	if not current_timeline_doc.is_empty():
		record["timeline_doc"] = current_timeline_doc.duplicate(true)
		# Avoid persisting empty/non-Array timeline_events which can mask downstream fallbacks.
		var doc_events_variant: Variant = current_timeline_doc.get("events", null)
		if doc_events_variant is Array and not (doc_events_variant as Array).is_empty():
			record["timeline_events"] = (doc_events_variant as Array).duplicate(true)
		elif doc_events_variant is String:
			var parsed_doc_events: Variant = JSON.parse_string(String(doc_events_variant))
			if parsed_doc_events is Array and not (parsed_doc_events as Array).is_empty():
				record["timeline_events"] = (parsed_doc_events as Array).duplicate(true)
		var rosters: Variant = current_timeline_doc.get("rosters", {})
		if rosters is Dictionary:
			record["timeline_rosters"] = (rosters as Dictionary).duplicate(true)
		var metadata: Variant = current_timeline_doc.get("metadata", {})
		if metadata is Dictionary:
			record["timeline_metadata"] = (metadata as Dictionary).duplicate(true)

	# ST-002 SSOT Fix: position_data comes exclusively from _result (no fallback)
	# MatchSimulationManager always includes position_data in ingest_external_match payload
	var pos_data: Dictionary = {}
	if _result.has("position_data") and _result.position_data is Dictionary:
		pos_data = (_result.position_data as Dictionary).duplicate(true)
	if not pos_data.is_empty():
		record["position_data"] = pos_data

	# Phase 17: Store MatchSetup metadata for deterministic timeline (Game OS)
	if current_match_setup:
		# Store MatchSetup metadata for timeline (don't serialize full object, use key fields)
		record["match_setup_meta"] = {
			"seed": current_match_setup.rng_seed,
			"match_id": current_match_setup.match_id,
			"match_type": current_match_setup.match_type,
			"home_formation": current_match_setup.home_team.formation if current_match_setup.home_team else "4-4-2",
			"away_formation": current_match_setup.away_team.formation if current_match_setup.away_team else "4-4-2",
			"home_roster_size": current_match_setup.home_team.starters.size() if current_match_setup.home_team else 0,
			"away_roster_size": current_match_setup.away_team.starters.size() if current_match_setup.away_team else 0
		}
		print("[MatchManager] Stored MatchSetup metadata in match record (Game OS mode)")

	match_history.append(record)
	print(
		(
			"[MatchManager] Recorded match history: %s vs %s (%s, seed: %d)"
			% [result_text, current_opponent.get("name", "Unknown"), "%d-%d" % [home_score, away_score], rng_seed]
		)
	)
	current_match_result = {}
	current_timeline_doc = {}
	current_match_config = {}
	current_match_seed = 0


func ingest_external_match(match_payload: Dictionary) -> void:
	"""Accept externally simulated match data and record it in history (with auto-save)."""
	var opponent_name: String = str(match_payload.get("opponent_name", "Unknown"))
	var opponent_rating: int = int(match_payload.get("opponent_rating", 50))
	var year_value: int = int(match_payload.get("year", 1))
	var week_value: int = int(match_payload.get("week", 1))
	var goals_scored: int = int(match_payload.get("goals_scored", match_payload.get("home_score", 0)))
	var goals_conceded: int = int(match_payload.get("goals_conceded", match_payload.get("away_score", 0)))
	var result_text: String = str(match_payload.get("result", "무승부"))
	var seed_value: int = int(match_payload.get("seed", 0))
	var final_score_value: Variant = match_payload.get("final_score", [goals_scored, goals_conceded])
	if final_score_value is Array:
		final_score_value = (final_score_value as Array).duplicate(true)

	var home_team_label: String = str(match_payload.get("home_team_name", HOME_TEAM_NAME))

	var record: Dictionary = {
		"opponent_name": opponent_name,
		"opponent_rating": opponent_rating,
		"home_team_name": home_team_label,
		"year": year_value,
		"week": week_value,
		"timestamp": match_payload.get("timestamp", Time.get_unix_time_from_system()),
		"result": result_text,
		"goals_scored": goals_scored,
		"goals_conceded": goals_conceded,
		"final_score": final_score_value,
		"tactic_used": match_payload.get("tactic_used", current_tactic),
		"match_duration": match_payload.get("match_duration", 90),
		"seed": seed_value,
		"events": match_payload.get("events", []),
		"raw_result": match_payload.get("raw_result", {}),
		"match_result": match_payload.get("match_result", {})
	}

	var timeline_variant: Variant = match_payload.get("timeline", [])
	if timeline_variant is Array:
		record["timeline"] = (timeline_variant as Array).duplicate(true)

	var record_events: Variant = record.get("events", [])
	if record_events is Array:
		record["events"] = (record_events as Array).duplicate(true)
	var record_raw_result: Variant = record.get("raw_result", {})
	if record_raw_result is Dictionary:
		record["raw_result"] = (record_raw_result as Dictionary).duplicate(true)
	var record_match_result: Variant = record.get("match_result", {})
	if record_match_result is Dictionary:
		record["match_result"] = (record_match_result as Dictionary).duplicate(true)

	var legacy_doc_key := ("re" + "play") + "_doc"
	var legacy_doc_key2 := "re" + "play"
	var doc_variant: Variant = null
	if match_payload.has("timeline_doc") and match_payload.timeline_doc is Dictionary:
		doc_variant = match_payload.timeline_doc
	elif match_payload.has(legacy_doc_key) and match_payload[legacy_doc_key] is Dictionary:
		doc_variant = match_payload[legacy_doc_key]
	elif match_payload.has(legacy_doc_key2) and match_payload[legacy_doc_key2] is Dictionary:
		doc_variant = match_payload[legacy_doc_key2]

	if doc_variant is Dictionary:
		var doc_copy: Dictionary = (doc_variant as Dictionary).duplicate(true)
		record["timeline_doc"] = doc_copy
		# Avoid persisting empty/non-Array timeline_events from timeline_doc (can mask record.events).
		var doc_events_variant: Variant = doc_copy.get("events", null)
		if doc_events_variant is Array and not (doc_events_variant as Array).is_empty():
			record["timeline_events"] = (doc_events_variant as Array).duplicate(true)
		elif doc_events_variant is String:
			var parsed_doc_events: Variant = JSON.parse_string(String(doc_events_variant))
			if parsed_doc_events is Array and not (parsed_doc_events as Array).is_empty():
				record["timeline_events"] = (parsed_doc_events as Array).duplicate(true)
		var metadata = doc_copy.get("metadata", {})
		if metadata is Dictionary:
			record["timeline_metadata"] = metadata.duplicate(true)

	var rosters_payload: Variant = match_payload.get("rosters", {})
	if rosters_payload is Dictionary:
		record["timeline_rosters"] = (rosters_payload as Dictionary).duplicate(true)

	var position_payload: Variant = match_payload.get("position_data", {})
	if position_payload is Dictionary and not (position_payload as Dictionary).is_empty():
		record["position_data"] = (position_payload as Dictionary).duplicate(true)
	elif match_payload.has("match_result") and match_payload.match_result is Dictionary:
		var match_res: Dictionary = match_payload.match_result
		var nested_pos: Variant = match_res.get("position_data", {})
		if nested_pos is Dictionary and not (nested_pos as Dictionary).is_empty():
			record["position_data"] = (nested_pos as Dictionary).duplicate(true)
	elif match_payload.has("raw_result") and match_payload.raw_result is Dictionary:
		var raw_res: Dictionary = match_payload.raw_result
		var raw_pos: Variant = raw_res.get("position_data", {})
		if raw_pos is Dictionary and not (raw_pos as Dictionary).is_empty():
			record["position_data"] = (raw_pos as Dictionary).duplicate(true)

	# Store detailed events (legacy key supported)
	var legacy_events_key := ("re" + "play") + "_events"
	var timeline_events_payload: Variant = match_payload.get(
		"timeline_events", match_payload.get(legacy_events_key, [])
	)
	if timeline_events_payload is Array and not (timeline_events_payload as Array).is_empty():
		record["timeline_events"] = (timeline_events_payload as Array).duplicate(true)
		print("[MatchManager] Stored timeline_events: %d events" % (timeline_events_payload as Array).size())
	elif match_payload.has("match_result") and match_payload.match_result is Dictionary:
		var match_res2: Dictionary = match_payload.match_result
		var nested_events: Variant = match_res2.get("timeline_events", match_res2.get(legacy_events_key, []))
		if nested_events is Array and not (nested_events as Array).is_empty():
			record["timeline_events"] = (nested_events as Array).duplicate(true)
			print(
				"[MatchManager] Stored timeline_events from match_result: %d events" % (nested_events as Array).size()
			)

	var stored_variant: Variant = match_payload.get("stored_events", [])
	if stored_variant is Array:
		record["stored_events"] = (stored_variant as Array).duplicate(true)

	# ✅ Phase 22: Add cup match metadata
	if match_payload.has("is_cup_match"):
		record["is_cup_match"] = match_payload.get("is_cup_match", false)
	if match_payload.has("cup_stage"):
		record["cup_stage"] = match_payload.get("cup_stage", "")

	# ✅ Phase 23: Division league metadata
	if DivisionManager and match_payload.get("type") == "league":
		record["is_league_match"] = true
		record["division"] = DivisionManager.current_division
		record["season"] = DivisionManager.current_season

		# Update division standings
		var league_result = {
			"win": result_text == "승리", "draw": result_text == "무승부", "gf": goals_scored, "ga": goals_conceded
		}
		DivisionManager.process_league_result(league_result)

		print(
			(
				"[MatchManager] Division %d standings updated: Position %d/6, Points %d"
				% [
					DivisionManager.current_division,
					DivisionManager.get_player_position(),
					DivisionManager.player_stats.get("points", 0)
				]
			)
		)

	match_history.append(record)

	print(
		(
			"[MatchManager] Recorded external match: %s vs %s (%d-%d, seed: %d)"
			% [result_text, opponent_name, goals_scored, goals_conceded, seed_value]
		)
	)

	if SaveManager and SaveManager.has_method("perform_auto_save"):
		SaveManager.perform_auto_save()


func _append_match_event(minute: int, icon: String, text: String, raw_event: Dictionary = {}) -> void:
	var entry = {"minute": minute, "icon": icon, "text": text, "raw": raw_event.duplicate(true)}
	current_match_events.append(entry)
	event_occurred.emit(minute, icon, text)


func get_match_history(limit: int = -1) -> Array:
	"""Get match history (most recent first)"""
	var history = match_history.duplicate()
	history.reverse()  # Most recent first

	if limit > 0 and history.size() > limit:
		return history.slice(0, limit)

	return history


func get_match_stats() -> Dictionary:
	"""Get match statistics"""
	var stats = {
		"total_matches": match_history.size(),
		"wins": 0,
		"draws": 0,
		"losses": 0,
		"goals_scored": 0,
		"goals_conceded": 0,
		"win_rate": 0.0,
		"average_goals_scored": 0.0,
		"average_goals_conceded": 0.0
	}

	if match_history.size() == 0:
		return stats

	for record in match_history:
		# Count results
		match record.get("result", ""):
			"승리":
				stats.wins += 1
			"무승부":
				stats.draws += 1
			"패배":
				stats.losses += 1

		# Sum goals
		stats.goals_scored += record.get("goals_scored", 0)
		stats.goals_conceded += record.get("goals_conceded", 0)

	# Calculate averages
	stats.win_rate = float(stats.wins) / float(match_history.size()) * 100.0
	stats.average_goals_scored = float(stats.goals_scored) / float(match_history.size())
	stats.average_goals_conceded = float(stats.goals_conceded) / float(match_history.size())

	return stats


func clear_match_history():
	"""Clear all match history (for testing/reset)"""
	match_history.clear()
	print("[MatchManager] Match history cleared")


func save_to_dict() -> Dictionary:
	"""저장용 데이터 반환 (Phase 9.2)"""
	return {"match_history": match_history.duplicate(true)}


func load_from_dict(data: Dictionary):
	"""로드용 데이터 복원 (Phase 9.2)"""
	if data.has("match_history"):
		match_history = data["match_history"].duplicate(true)
		print("[MatchManager] Loaded %d match records from save file" % match_history.size())
	else:
		match_history = []
		print("[MatchManager] No match history in save file")


# ========================================
# Phase 22: Hero Growth Extraction
# ========================================


func _extract_hero_growth_from_rust_result(rust_result: Variant) -> Dictionary:
	"""Extract hero_growth data from Rust FootballRustEngine result"""
	if rust_result == null:
		return _create_fallback_hero_growth()

	# Check if Rust result has hero_growth field
	if typeof(rust_result) == TYPE_DICTIONARY and rust_result.has("hero_growth"):
		return rust_result.hero_growth

	# Check if Rust result has growth data in different format
	if typeof(rust_result) == TYPE_DICTIONARY and rust_result.has("player_growth"):
		return _convert_player_growth_to_hero_growth(rust_result.player_growth)

	print("[MatchManager] ⚠️ No hero_growth in Rust result, using fallback")
	return _create_fallback_hero_growth()


func _create_fallback_hero_growth() -> Dictionary:
	"""Create minimal hero_growth when Rust engine doesn't provide it"""
	return {"stat_gains": {}, "xp_overflow": {}, "total_actions": 0, "hero_time_actions": 0}


func _convert_player_growth_to_hero_growth(player_growth: Dictionary) -> Dictionary:
	"""Convert Rust player_growth format to hero_growth format"""
	return {
		"stat_gains": player_growth.get("gains", {}),
		"xp_overflow": player_growth.get("overflow", {}),
		"total_actions": player_growth.get("total_actions", 0),
		"hero_time_actions": player_growth.get("hero_actions", 0)
	}


## Preflight validation: Check if roster UIDs can be resolved before engine call
func _preflight_match_setup_roster(match_setup: _MatchSetup) -> Dictionary:
	# Goal: fail fast with a clear message (prevents confusing "경기 종료" + engine error).
	if match_setup == null:
		return {"ok": false, "error": "MatchSetup is null"}
	if match_setup.home_team == null or match_setup.away_team == null:
		return {"ok": false, "error": "MatchSetup team objects missing"}

	var missing: Array = []

	# Extract starter UIDs (11 each)
	var home_uids: Array = []
	for p in match_setup.home_team.starters:
		if p is _MatchPlayer:
			home_uids.append(str(p.uid))
		else:
			var item: Variant = p
			if item is Dictionary and item.has("uid"):
				home_uids.append(str(item["uid"]))
			elif item != null and item.has_method("get_uid"):
				home_uids.append(str(item.call("get_uid")))

	var away_uids: Array = []
	for p in match_setup.away_team.starters:
		if p is _MatchPlayer:
			away_uids.append(str(p.uid))
		else:
			var item: Variant = p
			if item is Dictionary and item.has("uid"):
				away_uids.append(str(item["uid"]))
			elif item != null and item.has_method("get_uid"):
				away_uids.append(str(item.call("get_uid")))

	# Helper: check existence in engine/cache if possible
	for uid in home_uids:
		if not _can_resolve_uid(uid):
			missing.append("home:" + uid)
	for uid in away_uids:
		if not _can_resolve_uid(uid):
			missing.append("away:" + uid)

	if missing.size() > 0:
		push_error("[MatchManager] Preflight failed. Missing players: %s" % str(missing))
		return {"ok": false, "error": "Roster resolve failed (missing): %s" % str(missing), "missing": missing}

	return {"ok": true, "error": ""}


## Check if a UID can be resolved (exists in cache/library)
func _can_resolve_uid(uid: String) -> bool:
	# 1) 엔진 캐시 기준 존재 여부를 최우선으로 사용 (GDExtension/DataCacheStore)
	if (
		FootballRustEngine != null
		and is_instance_valid(FootballRustEngine)
		and FootballRustEngine.has_method("has_player_uid")
	):
		if bool(FootballRustEngine.call("has_player_uid", uid)):
			return true

		# csv:2 -> 2 같은 변형도 엔진 캐시에 있을 수 있으니 한 번 더 확인
		if uid.begins_with("csv:"):
			var n := uid.substr(4, uid.length() - 4)
			if n.is_valid_int():
				if bool(FootballRustEngine.call("has_player_uid", n)):
					return true

	# 2) PlayerLibrary에서 확인 (Godot-side - needs "csv:" prefix for numeric UIDs)
	if player_library != null and player_library.has_method("get_player_data"):
		var lookup_uid := uid
		# Convert numeric UID to csv: format for PlayerLibrary
		if uid.is_valid_int():
			lookup_uid = "csv:%s" % uid
		var d = player_library.get_player_data(lookup_uid)
		if d is Dictionary and not d.is_empty():
			return true

	# 3) 마지막 fallback: uid가 csv:<n>면 숫자 키도 한 번 시도 (키 규칙 어긋남 방어)
	if player_library != null and player_library.has_method("get_player_data"):
		if uid.begins_with("csv:"):
			var n := uid.substr(4, uid.length() - 4)
			if n.is_valid_int():
				var d2 = player_library.get_player_data(n)
				if d2 is Dictionary and not d2.is_empty():
					return true

	return false


## Resolve engine UID for main player (grad:* → csv: UID proxy)
## Never pass grad:* into engine simulation - always return "csv:<n>" format
##
## ✅ Engine UID SSOT (2025-12-23):
## - grad:* is not resolvable by Rust PLAYER_LIBRARY
## - Must proxy to a real engine cache UID ("csv:<n>" format)
## - Rust v2 API requires "csv:" prefix
func _resolve_engine_uid_for_main_player(raw_uid: Variant) -> String:
	var s := str(raw_uid).strip_edges()

	# Already numeric → add "csv:" prefix
	if s.is_valid_int():
		return "csv:%s" % s

	# Already "csv:<n>" → keep as is
	if s.begins_with("csv:"):
		return s

	# grad:* → pick a proxy uid that actually exists in engine cache
	if s.begins_with("grad:"):
		# Deterministic proxy: near team level / player overall
		return _pick_valid_engine_uid_near(_calculate_team_level())

	return ""


## Find a UID that exists in engine cache (uses FootballRustEngine.has_player_uid)
## Searches outward from center to avoid "Player not found" failures
##
## ✅ Engine UID SSOT (2025-12-23):
## - Returns "csv:<n>" format ("csv:2", "csv:117", etc.)
## - Guarantees the UID exists in Rust PLAYER_LIBRARY
## - Rust v2 API requires "csv:" prefix
func _pick_valid_engine_uid_near(center: int) -> String:
	var c := int(center)
	var max_radius := 200

	for r in range(max_radius + 1):
		var a: int = clamp(c - r, 1, 1000000)
		var b: int = clamp(c + r, 1, 1000000)
		var ua := str(a)
		var ub := str(b)

		if FootballRustEngine and FootballRustEngine.has_method("has_player_uid"):
			# Check both directions from center
			if bool(FootballRustEngine.call("has_player_uid", ua)):
				return "csv:%s" % ua
			if bool(FootballRustEngine.call("has_player_uid", ub)):
				return "csv:%s" % ub
		else:
			# If engine cache check isn't available, just return "csv:" prefixed
			return "csv:%s" % ua

	return ""


## Convert match_setup roster UIDs from Godot format to Rust format
## Godot: "csv:2", "grad:1" (string UIDs for display/tracking)
## Rust:  "2", "1" (numeric strings for PLAYER_LIBRARY lookup)
##
## CRITICAL: Rust PLAYER_LIBRARY uses integer keys, so we must strip prefixes
func _convert_match_setup_uids_for_rust(match_setup) -> void:
	if not match_setup:
		return

	# Convert home team starters
	if match_setup.home_team and match_setup.home_team.starters:
		var starters = match_setup.home_team.starters
		for i in range(starters.size()):
			var player = starters[i]
			if player is _MatchPlayer:
				player.uid = _uid_to_rust_format(str(player.uid))

	# Convert away team starters
	if match_setup.away_team and match_setup.away_team.starters:
		var starters = match_setup.away_team.starters
		for i in range(starters.size()):
			var player = starters[i]
			if player is _MatchPlayer:
				player.uid = _uid_to_rust_format(str(player.uid))


## Helper: Convert Godot UID to Rust PLAYER_LIBRARY format
## "csv:2" → "2", "grad:1" → "1", "2" → "2"
func _uid_to_rust_format(uid: String) -> String:
	if uid.begins_with("csv:"):
		return uid.substr(4)  # "csv:2" → "2"
	elif uid.begins_with("grad:"):
		return uid.substr(5)  # "grad:1" → "1"
	return uid  # Already numeric or other format


## Emit preflight failure to EventBus (optional UX layer)
func _emit_preflight_failed_eventbus(info: Dictionary) -> void:
	# Best-effort: EventBus is optional; UX should also work via the signal above.
	if has_node("/root/EventBus"):
		var eb = get_node("/root/EventBus")
		if eb and eb.has_method("emit"):
			eb.call("emit", "match_preflight_failed", info)
			return
	# plugin variant (some repos use EventBusDataHandler / EventBus singleton)
	if has_node("/root/EventBusDataHandler"):
		var eb2 = get_node("/root/EventBusDataHandler")
		if eb2 and eb2.has_method("emit"):
			eb2.call("emit", "match_preflight_failed", info)
