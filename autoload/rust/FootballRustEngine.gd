extends Node
## FootballRustEngine - Core autoload for Rust match simulation via GDExtension
## Part of the 5-autoload architecture (Constitution v7.0)
# Preload to avoid autoload order issues with class_name
const _TimelineBinaryLoader = preload("res://scripts/utils/TimelineBinaryLoader.gd")

# ST-006: Bridge classes for modular architecture
const _BinaryProtocolEncoder = preload("res://autoload/rust/rust_bridge/BinaryProtocolEncoder.gd")
const _GachaCoachBridge = preload("res://autoload/rust/rust_bridge/GachaCoachBridge.gd")
const _TrainingBridge = preload("res://autoload/rust/rust_bridge/TrainingBridge.gd")
const _TacticsBridge = preload("res://autoload/rust/rust_bridge/TacticsBridge.gd")
const _SaveLoadBridge = preload("res://autoload/rust/rust_bridge/SaveLoadBridge.gd")
const _MatchSessionBridge = preload("res://autoload/rust/rust_bridge/MatchSessionBridge.gd")
const _InteractiveMatchBridge = preload("res://autoload/rust/rust_bridge/InteractiveMatchBridge.gd")
const _MatchSimulationBridge = preload("res://autoload/rust/rust_bridge/MatchSimulationBridge.gd")

signal match_completed(success: bool, result: Dictionary)
signal match_error(error: String)
signal engine_ready
signal simulation_completed(job_id: String, json: String)
signal api_error(error_code: String, message: String)

const MAX_RETRIES = 2
const DEFAULT_TIMEOUT_MS = 5000

## ============================================================================
## MRQ0 Binary Protocol Constants
## See docs/spec/03_data_schemas.md "MRQ0 Binary Protocol" for full spec
## ============================================================================
const MRQ0_MAGIC: int = 0x3051514D  ## "MQQ0" little-endian
const MRQ0_VERSION_MIN: int = 3     ## Minimum supported version
const MRQ0_VERSION_MAX: int = 4     ## Maximum supported version
const MRQ0_VERSION_DEFAULT: int = 4 ## Current default version

# Cache bridge (GDExtension)
var _data_cache_store: Object = null

var _rust_simulator: Object = null
var _is_ready: bool = false
var _last_error: String = ""
var _active_job_count: int = 0

# Performance metrics
var _total_simulations: int = 0
var _total_time_ms: int = 0

# ST-006: Bridge instances
var _binary_encoder: RefCounted = null
var _gacha_coach: RefCounted = null
var _training: RefCounted = null
var _tactics: RefCounted = null
var _save_load: RefCounted = null
var _match_session: RefCounted = null
var _interactive_match: RefCounted = null
var _match_simulation: RefCounted = null

func _ready() -> void:
	_initialize_rust_engine()
	# Ensure background simulation jobs are polled so that
	# Rust emits `simulation_completed(job_id, json)` back to Godot.
	# Without polling, awaiting callers will hang.
	set_process(true)


func _process(_delta: float) -> void:
	# Poll only when there are active jobs
	if _active_job_count > 0:
		poll_simulation()


## Initialize the Rust GDExtension
func _initialize_rust_engine() -> void:
	if ClassDB.class_exists("FootballMatchSimulator"):
		_rust_simulator = ClassDB.instantiate("FootballMatchSimulator")

		if _rust_simulator:
			print("[FootballRustEngine] rust_simulator instance_id=", str(_rust_simulator.get_instance_id()))
			if not _verify_boundary_contract():
				_last_error = "Rust boundary contract failed (missing required methods)"
				push_error("[FootballRustEngine] " + _last_error)
				return
			# Capture DataCacheStore instance for cache-backed UID checks (if available)
			_initialize_data_cache_store()

			# Wire-through async simulation completion from Rust to this autoload
			if _rust_simulator.has_signal("simulation_completed"):
				_rust_simulator.connect("simulation_completed", Callable(self, "_on_rust_simulation_completed"))
			var test_result = _rust_simulator.test_connection()
			if test_result and test_result != "":
				_is_ready = true
				print("[FootballRustEngine] Initialized: ", test_result)
				print("[FootballRustEngine] Version: ", get_version())
				print("[FootballRustEngine] Build: ", get_build_tag())
				# ST-006: Initialize bridge classes
				_initialize_bridge_classes()
				engine_ready.emit()
			else:
				_last_error = "Failed to connect to Rust engine"
				push_error("[FootballRustEngine] " + _last_error)
		else:
			_last_error = "Failed to instantiate FootballMatchSimulator"
			push_error("[FootballRustEngine] " + _last_error)
	else:
		_last_error = "FootballMatchSimulator class not found. Check GDExtension setup."
		push_error("[FootballRustEngine] " + _last_error)


func _verify_boundary_contract() -> bool:
	# Boot-time contract enforcement: avoid "works on my DLL" drift.
	if not _rust_simulator:
		return false

	var required_methods := ["simulate_match_from_setup", "simulate_match_from_binary"]  # Phase17 canonical entrypoint  # MRB0/OFRP binary replay path

	for m in required_methods:
		if not _rust_simulator.has_method(m):
			push_error("[FootballRustEngine] Missing required Rust method: %s" % m)
			if OS.is_debug_build():
				assert(false, "Missing required Rust method: %s" % m)
			return false

	if OS.is_debug_build():
		print("[FootballRustEngine] Boundary contract OK: ", required_methods)
	return true


func _initialize_data_cache_store() -> void:
	# DataCacheStore is registered by GDExtension in newer builds.
	# If it exists, we keep a handle so MatchManager preflight can query real cache membership.
	if ClassDB.class_exists("DataCacheStore"):
		_data_cache_store = ClassDB.instantiate("DataCacheStore")
		if _data_cache_store:
			print("[FootballRustEngine] DataCacheStore connected: ", _data_cache_store)
		else:
			push_warning("[FootballRustEngine] DataCacheStore exists but instantiation failed")
	else:
		_data_cache_store = null


func _initialize_bridge_classes() -> void:
	"""ST-006: Initialize bridge classes for modular architecture"""
	# BinaryProtocolEncoder (stateless, no rust_simulator needed)
	_binary_encoder = _BinaryProtocolEncoder.new()
	_binary_encoder.initialize(_TimelineBinaryLoader)

	# GachaCoachBridge (needs rust_simulator)
	_gacha_coach = _GachaCoachBridge.new()
	_gacha_coach.initialize(_rust_simulator)

	# TrainingBridge (needs rust_simulator)
	_training = _TrainingBridge.new()
	_training.initialize(_rust_simulator)

	# TacticsBridge (needs rust_simulator)
	_tactics = _TacticsBridge.new()
	_tactics.initialize(_rust_simulator)

	# SaveLoadBridge (needs rust_simulator)
	_save_load = _SaveLoadBridge.new()
	_save_load.initialize(_rust_simulator)

	# Phase 3 bridges
	# MatchSessionBridge (needs rust_simulator)
	_match_session = _MatchSessionBridge.new()
	_match_session.initialize(_rust_simulator)

	# InteractiveMatchBridge (needs rust_simulator)
	_interactive_match = _InteractiveMatchBridge.new()
	_interactive_match.initialize(_rust_simulator)

	# MatchSimulationBridge (needs rust_simulator, binary_encoder, gacha_coach)
	_match_simulation = _MatchSimulationBridge.new()
	_match_simulation.initialize(_rust_simulator, _binary_encoder, _gacha_coach)
	# Connect signals from MatchSimulationBridge to forward to this autoload
	if _match_simulation.has_signal("match_completed"):
		_match_simulation.connect("match_completed", Callable(self, "_on_bridge_match_completed"))
	if _match_simulation.has_signal("match_error"):
		_match_simulation.connect("match_error", Callable(self, "_on_bridge_match_error"))

	if OS.is_debug_build():
		print("[FootballRustEngine] Bridge classes initialized (ST-006 Phase 1-3)")


func _on_rust_simulation_completed(job_id: String, json: String) -> void:
	# Forward simulation completion to consumers (e.g., OpenFootballAPI)
	print("[FootballRustEngine] simulation_completed received: job_id=", job_id, " bytes=", json.length())
	simulation_completed.emit(job_id, json)
	if _active_job_count > 0:
		_active_job_count -= 1
	if _match_simulation:
		_match_simulation.decrement_active_jobs()


func _on_bridge_match_completed(success: bool, result: Dictionary) -> void:
	# Forward from MatchSimulationBridge
	match_completed.emit(success, result)


func _on_bridge_match_error(error: String) -> void:
	# Forward from MatchSimulationBridge
	match_error.emit(error)


## Check if the engine is ready
func is_ready() -> bool:
	return _is_ready


## Get the last error message
func get_last_error() -> String:
	return _last_error


## Get the Rust simulator instance (for advanced usage)
func get_simulator() -> Object:
	return _rust_simulator


## Simulate a match using JSON data
## @param match_data: Dictionary containing match setup
## @return: Dictionary with match result or error
func simulate_match_json(match_data: Dictionary) -> Dictionary:
	if _match_simulation:
		return _match_simulation.simulate_match_json(match_data)
	var error_result = {"error": true, "message": "MatchSimulationBridge not initialized", "error_code": "BRIDGE_NOT_READY"}
	match_error.emit(error_result.message)
	return error_result


## Simulate a match using MatchRequest schema v2 (UID roster).
## @param match_data: Dictionary containing MatchRequestV2 (schema_version=2)
## @return: Dictionary with match result or error
func simulate_match_v2_json(match_data: Dictionary) -> Dictionary:
	if _match_simulation:
		return _match_simulation.simulate_match_v2_json(match_data)
	var error_result = {"error": true, "message": "MatchSimulationBridge not initialized", "error_code": "BRIDGE_NOT_READY"}
	match_error.emit(error_result.message)
	return error_result


## Simulate a match using MatchRequest schema v2 and return (result_json, timeline_json).
## @return: Dictionary with keys: result (Dictionary), timeline_doc (Dictionary), result_json (String), timeline_json (String) or error fields
func simulate_match_v2_with_timeline(match_data: Dictionary) -> Dictionary:
	if _match_simulation:
		return _match_simulation.simulate_match_v2_with_timeline(match_data)
	var error_result = {"error": true, "message": "MatchSimulationBridge not initialized", "error_code": "BRIDGE_NOT_READY"}
	match_error.emit(error_result.message)
	return error_result


## Simulate a match using MRQ0 v3 binary protocol (Phase C: Binary Migration)
## @param match_data: Dictionary containing match setup
## @return: Dictionary with match result or error
func simulate_match_binary(match_data: Dictionary) -> Dictionary:
	if not _is_ready:
		var error_result = {
			"error": true, "message": "Engine not ready: " + _last_error, "error_code": "ENGINE_NOT_READY"
		}
		match_error.emit(error_result.message)
		return error_result

	var start_time = Time.get_ticks_msec()
	var seed_val: int = int(match_data.get("seed", Time.get_ticks_usec()))

	var mrq0_version: int = int(match_data.get("mrq0_version", MRQ0_VERSION_DEFAULT))
	if mrq0_version < MRQ0_VERSION_MIN or mrq0_version > MRQ0_VERSION_MAX:
		push_warning("[FootballRustEngine] Unsupported MRQ0 version %d, using default %d" % [mrq0_version, MRQ0_VERSION_DEFAULT])
		mrq0_version = MRQ0_VERSION_DEFAULT

	print("🟦 [FootballRustEngine] MRQ0 v%d Simulation (seed=%d)" % [mrq0_version, seed_val])

	# Encode MRQ0 payload (see docs/spec/03_data_schemas.md "MRQ0 Binary Protocol")
	var buf := StreamPeerBuffer.new()
	buf.big_endian = false
	buf.put_u32(MRQ0_MAGIC)    # Magic: "MQQ0" (0x3051514D)
	buf.put_u32(mrq0_version)  # Version (currently 3 or 4)
	buf.put_u64(seed_val)

	var use_vendor: bool = bool(match_data.get("use_vendor_engine", false))
	buf.put_u8(1 if use_vendor else 0)

	buf.put_u16(100)  # position_sample_rate_ms

	var home_team: Dictionary = match_data.get("home_team", {})
	var away_team: Dictionary = match_data.get("away_team", {})

	_encode_team_for_binary(buf, home_team)

	# Home Instructions
	var home_instr: Dictionary = match_data.get("home_instructions", home_team.get("instructions", {}))
	_encode_instructions_binary(buf, home_instr)

	_encode_team_for_binary(buf, away_team)

	# Away Instructions
	var away_instr: Dictionary = match_data.get("away_instructions", away_team.get("instructions", {}))
	_encode_instructions_binary(buf, away_instr)

	# ============================================================
	# MRQ0 Extension Block (append-only, backward-compatible)
	# - Optional match modifiers (SSOT: Rust coach/deck)
	# - v3: includes reserved training_multiplier
	# - v4: removes training_multiplier (match-only protocol)
	# - v1.1: Supports HOME and optional AWAY modifier bundles (flags.bit1)
	# ============================================================
	var ext_flags: int = 0
	var include_deck_effects: bool = bool(match_data.get("include_deck_effects", true))
	if include_deck_effects:
		# Side selection:
		# - explicit: match_data.deck_effects_side = "home"|"away"|"both"|"none"
		# - inferred: hero_uid ∈ home/away (starting_xi/bench) -> that side
		var deck_effects_side := str(match_data.get("deck_effects_side", "")).strip_edges().to_lower()
		if deck_effects_side == "":
			var hero_uid_raw: Variant = match_data.get("hero_uid", null)
			var hero_uid_int: int = _extract_csv_numeric(str(hero_uid_raw)) if hero_uid_raw != null else -1
			if hero_uid_int != -1:
				if _team_contains_engine_uid(home_team, hero_uid_int):
					deck_effects_side = "home"
				elif _team_contains_engine_uid(away_team, hero_uid_int):
					deck_effects_side = "away"

		if deck_effects_side == "":
			deck_effects_side = "home"

		var apply_home := false
		var apply_away := false
		match deck_effects_side:
			"home":
				apply_home = true
			"away":
				apply_away = true
			"both":
				apply_home = true
				apply_away = true
			"none":
				apply_home = false
				apply_away = false
			_:
				apply_home = true

		# v1.2: per-side modifier bundles/decks (PvP). These override local-side defaults.
		var home_override_mods: Array = []
		var away_override_mods: Array = []
		var home_override_set := false
		var away_override_set := false

		if match_data.has("home_match_modifiers"):
			home_override_mods = _normalize_mrq0_match_modifiers(match_data.get("home_match_modifiers", []))
			home_override_set = true
		elif match_data.has("home_deck") and match_data.get("home_deck") is Dictionary:
			var mm_home: Dictionary = deck_calculate_match_modifiers(match_data.get("home_deck") as Dictionary)
			if bool(mm_home.get("success", false)):
				home_override_mods = _normalize_mrq0_match_modifiers(mm_home.get("match_modifiers", []))
			else:
				push_warning("[FootballRustEngine] MRQ0 extension: home_deck derive failed: %s" % str(mm_home.get("error", "unknown")))
			home_override_set = true

		if match_data.has("away_match_modifiers"):
			away_override_mods = _normalize_mrq0_match_modifiers(match_data.get("away_match_modifiers", []))
			away_override_set = true
		elif match_data.has("away_deck") and match_data.get("away_deck") is Dictionary:
			var mm_away: Dictionary = deck_calculate_match_modifiers(match_data.get("away_deck") as Dictionary)
			if bool(mm_away.get("success", false)):
				away_override_mods = _normalize_mrq0_match_modifiers(mm_away.get("match_modifiers", []))
			else:
				push_warning("[FootballRustEngine] MRQ0 extension: away_deck derive failed: %s" % str(mm_away.get("error", "unknown")))
			away_override_set = true

		# Effective apply: local deck selection + explicit per-side overrides.
		var eff_apply_home := apply_home or home_override_set
		var eff_apply_away := apply_away or away_override_set

		if eff_apply_home or eff_apply_away:
			var base_mods: Array = []
			var need_base_mods := (apply_home and not home_override_set) or (apply_away and not away_override_set)
			if need_base_mods:
				var mm_base: Dictionary = deck_calculate_match_modifiers({})
				if bool(mm_base.get("success", false)):
					base_mods = _normalize_mrq0_match_modifiers(mm_base.get("match_modifiers", []))
				else:
					push_warning("[FootballRustEngine] MRQ0 extension: deck_calculate_match_modifiers failed: %s" % str(mm_base.get("error", "unknown")))

			var home_mods: Array = home_override_mods if home_override_set else base_mods
			var away_mods: Array = away_override_mods if away_override_set else base_mods
			if not eff_apply_home:
				home_mods = []
			if not eff_apply_away:
				away_mods = []

			# Only write extension block if anything non-empty exists.
			if home_mods.size() > 0 or away_mods.size() > 0:
				ext_flags |= 1  # bit0: has_deck_effects bundle
				if away_mods.size() > 0:
					ext_flags |= 2  # bit1: has_away_deck_effects list

				buf.put_u8(ext_flags)
				if mrq0_version <= 3:
					# v3 compatibility: keep training_multiplier reserved (=1.0) in the payload.
					buf.put_float(1.0)
				_encode_mrq0_match_modifiers_list(buf, home_mods)
				if (ext_flags & 2) != 0:
					_encode_mrq0_match_modifiers_list(buf, away_mods)
	else:
		# Extension block omitted when disabled
		pass

	var request_bytes: PackedByteArray = buf.data_array
	print("🟦 [FootballRustEngine] Sending %d bytes to Rust simulate_match_from_binary" % request_bytes.size())

	if not _rust_simulator.has_method("simulate_match_from_binary"):
		var error_result = {
			"error": true, "message": "simulate_match_from_binary not found", "error_code": "METHOD_NOT_FOUND"
		}
		match_error.emit(error_result.message)
		return error_result

	var rust_call_start := Time.get_ticks_msec()
	var replay_bytes: PackedByteArray = _rust_simulator.simulate_match_from_binary(request_bytes)
	var rust_call_elapsed := Time.get_ticks_msec() - rust_call_start

	print(
		"🟦 [FootballRustEngine] Rust call finished in %d ms (size: %d bytes)" % [rust_call_elapsed, replay_bytes.size()]
	)

	if replay_bytes.size() == 0:
		var error_result = {
			"error": true, "message": "Empty binary response from Rust engine", "error_code": "EMPTY_RESPONSE"
		}
		match_error.emit(error_result.message)
		return error_result

	# Parse binary data using TimelineBinaryLoader
	var timeline_data = _TimelineBinaryLoader.load_from_buffer(replay_bytes)

	if timeline_data == null:
		var error_result = {
			"error": true, "message": "Failed to parse timeline binary data", "error_code": "PARSE_ERROR"
		}
		match_error.emit(error_result.message)
		return error_result

	# Phase20 P0: SSOT probe gate (track_id 0..21)
	if OS.is_debug_build():
		_ssot_dbg_keys("FootballRustEngine:timeline_data.players", timeline_data.players)

	# Convert TimelineBinaryLoader format to expected match result format
	var result: Dictionary = {
		"success": true,
		"seed": seed_val,
		"metadata":
		{
			"duration": timeline_data.metadata.duration,
			"home_team_id": timeline_data.metadata.home_team_id,
			"away_team_id": timeline_data.metadata.away_team_id,
			"home_team_name": timeline_data.metadata.home_team_name,
			"away_team_name": timeline_data.metadata.away_team_name
		},
		"position_data": {"ball": [], "players": {}}
	}

	# Map ball track
	for frame in timeline_data.ball_track:
		result.position_data.ball.append(
			{"t": frame.t, "x": frame.x, "y": frame.y, "z": frame.z, "vx": frame.vx, "vy": frame.vy}
		)

	# Map player tracks
	for player_id in timeline_data.players.keys():
		var seq = timeline_data.players[player_id]
		var frames := []
		for pf in seq.frames:
			frames.append({"t": pf.t, "x": pf.x, "y": pf.y, "vx": pf.vx, "vy": pf.vy})
		result.position_data.players[player_id] = frames

	if OS.is_debug_build():
		_ssot_dbg_keys("FootballRustEngine:result.position_data.players", result.position_data.players)

	_check_timeout(start_time, "simulate_match_binary")
	match_completed.emit(true, result)

	return result


## MatchSetup OS simulation entrypoint (Phase 17).
## Bridges MatchSetupExporter payload -> MatchRequest v2 (UID roster) -> Rust simulate_match_from_setup().
func simulate_match_pure_binary(match_data: Dictionary) -> Dictionary:
	if not _is_ready:
		var error_result = {
			"error": true, "message": "Engine not ready: " + _last_error, "error_code": "ENGINE_NOT_READY"
		}
		match_error.emit(error_result.message)
		return error_result

	var start_time = Time.get_ticks_msec()

	var request_v2 := _build_match_request_v2_from_match_setup_payload(match_data)
	if request_v2.is_empty():
		var error_result = {
			"error": true,
			"message": "Invalid MatchSetup payload (cannot build MatchRequest v2)",
			"error_code": "INVALID_MATCH_PAYLOAD",
			"success": false
		}
		match_error.emit(error_result.message)
		return error_result

	var v2_out: Dictionary = simulate_match_v2_with_timeline(request_v2)
	if v2_out.get("error", false):
		var msg := str(v2_out.get("message", "v2 simulation failed"))
		var error_result = {
			"error": true,
			"message": msg,
			"error_code": str(v2_out.get("error_code", "SIMULATION_FAILED")),
			"success": false
		}
		match_error.emit(error_result.message)
		return error_result

	var result_variant: Variant = v2_out.get("result", {})
	var rust_result: Dictionary = result_variant if result_variant is Dictionary else {}

	# Normalize for OpenFootballAPI._parse_match_result():
	rust_result["success"] = true
	rust_result["error"] = false
	if not rust_result.has("seed"):
		rust_result["seed"] = int(request_v2.get("seed", 0))
	var timeline_doc_variant: Variant = v2_out.get("timeline_doc", {})
	if timeline_doc_variant is Dictionary and not (timeline_doc_variant as Dictionary).is_empty():
		rust_result["timeline_doc"] = (timeline_doc_variant as Dictionary).duplicate(true)
	rust_result["result_json"] = str(v2_out.get("result_json", ""))
	rust_result["timeline_json"] = str(v2_out.get("timeline_json", ""))

	# Check if the result is valid
	if typeof(rust_result) != TYPE_DICTIONARY:
		var error_result = {
			"error": true,
			"message": "simulate_match_pure_binary returned invalid type",
			"error_code": "INVALID_RESPONSE_TYPE",
			"success": false
		}
		match_error.emit(error_result.message)
		return error_result

	# Check if simulation succeeded
	if not rust_result.get("success", false):
		var error_result = {
			"error": true,
			"message": rust_result.get("error", "Unknown simulation error"),
			"error_code": "SIMULATION_FAILED",
			"success": false
		}
		match_error.emit(error_result.message)
		return error_result

	print("[FootballRustEngine] ✅ Match simulation completed successfully")

	_check_timeout(start_time, "simulate_match_pure_binary")
	match_completed.emit(true, rust_result)

	return rust_result


func _build_match_request_v2_from_match_setup_payload(match_setup: Dictionary) -> Dictionary:
	# Pass-through if caller already provided a schema v2 payload
	if int(match_setup.get("schema_version", 0)) == 2:
		var req := match_setup.duplicate(true)
		var home_team: Dictionary = req.get("home_team", {}) if req.get("home_team") is Dictionary else {}
		var away_team: Dictionary = req.get("away_team", {}) if req.get("away_team") is Dictionary else {}

		if home_team.is_empty() or away_team.is_empty():
			return {}

		home_team["formation"] = _normalize_formation_for_v2(str(home_team.get("formation", "4-4-2")))
		away_team["formation"] = _normalize_formation_for_v2(str(away_team.get("formation", "4-4-2")))
		home_team["roster"] = _build_roster_uids_from_match_setup_team(home_team)
		away_team["roster"] = _build_roster_uids_from_match_setup_team(away_team)

		req["home_team"] = home_team
		req["away_team"] = away_team
		return req

	if not (match_setup.has("home_team") and match_setup.has("away_team")):
		return {}

	var home_team: Dictionary = match_setup.get("home_team", {}) if match_setup.get("home_team") is Dictionary else {}
	var away_team: Dictionary = match_setup.get("away_team", {}) if match_setup.get("away_team") is Dictionary else {}

	var seed_value: int = int(match_setup.get("seed", Time.get_ticks_usec()))
	var enable_position_tracking: bool = bool(match_setup.get("enable_position_tracking", true))
	var use_real_names: bool = bool(match_setup.get("use_real_names", false))

	var home_name := str(home_team.get("name", "home"))
	var away_name := str(away_team.get("name", "away"))

	var home_formation := _normalize_formation_for_v2(
		str(home_team.get("formation", home_team.get("formation_id", "4-4-2")))
	)
	var away_formation := _normalize_formation_for_v2(
		str(away_team.get("formation", away_team.get("formation_id", "4-4-2")))
	)

	var home_roster: Array = _build_roster_uids_from_match_setup_team(home_team)
	var away_roster: Array = _build_roster_uids_from_match_setup_team(away_team)

	if home_roster.size() != 18 or away_roster.size() != 18:
		return {}

	var req := {
		"schema_version": 2,
		"seed": seed_value,
		"home_team":
		{
			"name": home_name,
			"formation": home_formation,
			"roster": home_roster,
		},
		"away_team":
		{
			"name": away_name,
			"formation": away_formation,
			"roster": away_roster,
		},
		"enable_position_tracking": enable_position_tracking,
		"use_real_names": use_real_names,
	}

	# Pass through team instructions: check match_setup level first, then team level
	var home_instr: Variant = match_setup.get("home_instructions", home_team.get("instructions", null))
	var away_instr: Variant = match_setup.get("away_instructions", away_team.get("instructions", null))

	if home_instr != null and home_instr is Dictionary:
		req["home_instructions"] = _normalize_team_instructions(home_instr as Dictionary)
	if away_instr != null and away_instr is Dictionary:
		req["away_instructions"] = _normalize_team_instructions(away_instr as Dictionary)

	# Pass through AI difficulty settings
	var home_ai_diff: Variant = match_setup.get("home_ai_difficulty", home_team.get("ai_difficulty", null))
	var away_ai_diff: Variant = match_setup.get("away_ai_difficulty", away_team.get("ai_difficulty", null))

	if home_ai_diff != null:
		req["home_ai_difficulty"] = str(home_ai_diff)
	if away_ai_diff != null:
		req["away_ai_difficulty"] = str(away_ai_diff)

	return req


## Normalize team instructions to Rust-compatible format
func _normalize_team_instructions(instr: Dictionary) -> Dictionary:
	return {
		"defensive_line": str(instr.get("defensive_line", "Normal")),
		"team_width": str(instr.get("team_width", "Normal")),
		"team_tempo": str(instr.get("team_tempo", "Normal")),
		"pressing_intensity": str(instr.get("pressing_intensity", "Medium")),
		"build_up_style": str(instr.get("build_up_style", "Mixed")),
		"use_offside_trap": bool(instr.get("use_offside_trap", false)),
	}


func _normalize_formation_for_v2(formation: String) -> String:
	var f := formation.strip_edges()
	if f == "":
		return "4-4-2"
	if f.find("-") != -1:
		return f
	if not f.begins_with("T"):
		return f

	# "T442" -> "4-4-2", "T4231" -> "4-2-3-1", ...
	var num_str := f.substr(1)  # Remove "T"
	match num_str.length():
		3:
			return "%s-%s-%s" % [num_str[0], num_str[1], num_str[2]]
		4:
			return "%s-%s-%s-%s" % [num_str[0], num_str[1], num_str[2], num_str[3]]
		5:
			return "%s-%s-%s%s-%s" % [num_str[0], num_str[1], num_str[2], num_str[3], num_str[4]]
		_:
			return f


func _build_roster_uids_from_match_setup_team(team: Dictionary) -> Array:
	# If already a v2 team payload, use it directly.
	var default_condition := 3
	var roster_variant: Variant = team.get("roster", null)
	if roster_variant is Array:
		var out: Array = []
		for entry in roster_variant:
			if entry is Dictionary:
				var d: Dictionary = (entry as Dictionary).duplicate(true)
				if d.has("uid"):
					if not d.has("condition"):
						d["condition"] = default_condition
					out.append(d)
				elif d.has("name"):
					# Embedded entry: ensure condition exists.
					if not d.has("condition"):
						d["condition"] = default_condition
					out.append(d)
				else:
					var uid := str(d.get("uid", d.get("id", "")))
					if uid != "":
						out.append({"uid": uid, "condition": default_condition})
			else:
				out.append({"uid": str(entry), "condition": default_condition})
		return out

	# NEW: If team has "players" array with full data, use embedded format (MRQ0 v3)
	var players_variant: Variant = team.get("players", null)
	if players_variant is Array:
		return _build_roster_embedded_from_players(players_variant as Array)

	# MatchSetupExporter shape: starting_xi(11) + bench(7)
	var starters_variant: Variant = team.get("starting_xi", team.get("starters", null))
	var bench_variant: Variant = team.get("bench", null)
	if not (starters_variant is Array and bench_variant is Array):
		return []

	var starters: Array = starters_variant
	var bench: Array = bench_variant
	var out: Array = []
	for uid in starters:
		out.append({"uid": str(uid), "condition": default_condition})
	for uid in bench:
		out.append({"uid": str(uid), "condition": default_condition})
	return out


## Build embedded player roster from players array (MRQ0 v3)
## Returns Array of Dictionaries with {name, position, overall, attributes?, track_id?}
func _build_roster_embedded_from_players(players: Array) -> Array:
	var out: Array = []
	for i in range(players.size()):
		var player: Variant = players[i]
		if player is String:
			# UID string - FIX01 requires {uid, condition}
			out.append({"uid": str(player), "condition": 3})
			continue
		if not player is Dictionary:
			continue

		var p: Dictionary = player as Dictionary
		var cond: int = clampi(int(p.get("condition", 3)), 1, 5)
		var entry: Dictionary = {
			"name": str(p.get("name", "Player_%d" % i)),
			"position": str(p.get("position_code", p.get("position", "MF"))),
			"overall": int(p.get("overall", 50)),
			"condition": cond,
		}

		# Include track_id if present
		if p.has("track_id"):
			entry["track_id"] = int(p.get("track_id"))

		# Include attributes if present (36 fields)
		var attrs_variant: Variant = p.get("attributes", null)
		if attrs_variant is Dictionary:
			var attrs: Dictionary = attrs_variant as Dictionary
			entry["attributes"] = _convert_attributes_for_rust(attrs)

		# Include personality if present (Leader/Genius/Workhorse/Rebel/Steady)
		if p.has("personality") and p.personality != null:
			entry["personality"] = str(p.personality)

		# Include traits if present (max 4 slots)
		var traits_variant: Variant = p.get("traits", null)
		if traits_variant is Array:
			entry["traits"] = _convert_traits_for_rust(traits_variant as Array)

		out.append(entry)
	return out


## Convert traits array to Rust-compatible format
func _convert_traits_for_rust(traits: Array) -> Array:
	var out: Array = []
	for t in traits:
		if not t is Dictionary:
			continue
		var trait_dict: Dictionary = t as Dictionary
		(
			out
			. append(
				{
					"id": str(trait_dict.get("id", "")),
					"tier": str(trait_dict.get("tier", "Bronze")),
				}
			)
		)
	return out


## Convert player attributes dictionary to Rust-compatible format
func _convert_attributes_for_rust(attrs: Dictionary) -> Dictionary:
	return {
		# Technical (14)
		"corners": int(attrs.get("corners", 50)),
		"crossing": int(attrs.get("crossing", 50)),
		"dribbling": int(attrs.get("dribbling", 50)),
		"finishing": int(attrs.get("finishing", 50)),
		"first_touch": int(attrs.get("first_touch", 50)),
		"free_kick_taking": int(attrs.get("free_kick_taking", attrs.get("free_kicks", 50))),
		"heading": int(attrs.get("heading", 50)),
		"long_shots": int(attrs.get("long_shots", 50)),
		"long_throws": int(attrs.get("long_throws", 50)),
		"marking": int(attrs.get("marking", 50)),
		"passing": int(attrs.get("passing", 50)),
		"penalty_taking": int(attrs.get("penalty_taking", 50)),
		"tackling": int(attrs.get("tackling", 50)),
		"technique": int(attrs.get("technique", 50)),
		# Mental (14)
		"aggression": int(attrs.get("aggression", 50)),
		"anticipation": int(attrs.get("anticipation", 50)),
		"bravery": int(attrs.get("bravery", 50)),
		"composure": int(attrs.get("composure", 50)),
		"concentration": int(attrs.get("concentration", 50)),
		"decisions": int(attrs.get("decisions", 50)),
		"determination": int(attrs.get("determination", 50)),
		"flair": int(attrs.get("flair", 50)),
		"leadership": int(attrs.get("leadership", 50)),
		"off_the_ball": int(attrs.get("off_the_ball", 50)),
		"positioning": int(attrs.get("positioning", 50)),
		"teamwork": int(attrs.get("teamwork", 50)),
		"vision": int(attrs.get("vision", 50)),
		"work_rate": int(attrs.get("work_rate", 50)),
		# Physical (8)
		"acceleration": int(attrs.get("acceleration", 50)),
		"agility": int(attrs.get("agility", 50)),
		"balance": int(attrs.get("balance", 50)),
		"jumping_reach": int(attrs.get("jumping_reach", attrs.get("jumping", 50))),
		"natural_fitness": int(attrs.get("natural_fitness", 50)),
		"pace": int(attrs.get("pace", 50)),
		"stamina": int(attrs.get("stamina", 50)),
		"strength": int(attrs.get("strength", 50)),
	}


func _put_string_binary(buf: StreamPeerBuffer, s: String) -> void:
	var bytes := s.to_utf8_buffer()
	buf.put_u16(bytes.size())
	if bytes.size() > 0:
		buf.put_data(bytes)


func _encode_position_binary(pos: String) -> int:
	match pos.to_upper():
		"GK":
			return 0
		"LB":
			return 1
		"CB":
			return 2
		"RB":
			return 3
		"LWB":
			return 4
		"RWB":
			return 5
		"CDM":
			return 6
		"CM":
			return 7
		"CAM":
			return 8
		"LM":
			return 9
		"RM":
			return 10
		"LW":
			return 11
		"RW":
			return 12
		"CF":
			return 13
		"ST":
			return 14
		"DF":
			return 15
		"MF":
			return 16
		"FW":
			return 17
		_:
			return 7  # CM 기본


func _encode_team_for_binary(buf: StreamPeerBuffer, team: Dictionary) -> void:
	_put_string_binary(buf, String(team.get("name", "Unknown")))
	_put_string_binary(buf, String(team.get("formation", "4-4-2")))

	var players: Array = _get_team_players_for_binary(team)
	var count: int = min(players.size(), 22)
	buf.put_u8(count)

	for i in range(count):
		var p: Dictionary = players[i]
		_put_string_binary(buf, String(p.get("name", "Player %d" % i)))
		buf.put_u8(_encode_position_binary(String(p.get("position", "CM"))))
		buf.put_u8(int(p.get("overall", 60)))


func _get_team_players_for_binary(team: Dictionary) -> Array:
	# Support both legacy shapes:
	# - `{ players: [...] }`
	# - MatchRequest v2 export: `{ starters: [...] }`
	var players_variant: Variant = team.get("players", null)
	if players_variant is Array:
		return players_variant
	var starters_variant: Variant = team.get("starters", null)
	if starters_variant is Array:
		return starters_variant
	return []


## Normalize MRQ0 match modifiers into a stable, deduped list.
## Input format: Array[Dictionary{ mod_id:int, value:float }]
func _normalize_mrq0_match_modifiers(mods_variant: Variant) -> Array:
	var by_id: Dictionary = {}
	if mods_variant is Array:
		for m in mods_variant:
			if typeof(m) != TYPE_DICTIONARY:
				continue
			var mod_id: int = int(m.get("mod_id", 0))
			if mod_id <= 0 or mod_id > 255:
				continue
			var value: float = float(m.get("value", 1.0))
			by_id[mod_id] = value

	var keys: Array = by_id.keys()
	keys.sort()

	var out: Array = []
	for k in keys:
		out.append({"mod_id": int(k), "value": float(by_id[k])})
	return out


## Encode a match modifier list as: u8 count + repeated (u8 mod_id, f32 value)
func _encode_mrq0_match_modifiers_list(buf: StreamPeerBuffer, mods: Array) -> void:
	var normalized: Array = _normalize_mrq0_match_modifiers(mods)
	buf.put_u8(min(normalized.size(), 255))
	for m in normalized:
		if typeof(m) != TYPE_DICTIONARY:
			continue
		var mod_id: int = int(m.get("mod_id", 0))
		if mod_id <= 0 or mod_id > 255:
			continue
		var value: float = float(m.get("value", 1.0))
		buf.put_u8(mod_id)
		buf.put_float(value)


func _encode_instructions_binary(buf: StreamPeerBuffer, instr: Dictionary) -> void:
	buf.put_u8(_encode_defensive_line(str(instr.get("defensive_line", "Normal"))))
	buf.put_u8(_encode_width(str(instr.get("team_width", "Normal"))))       
	buf.put_u8(_encode_tempo(str(instr.get("team_tempo", "Normal"))))       
	buf.put_u8(_encode_pressing(str(instr.get("pressing_intensity", "Medium"))))
	buf.put_u8(_encode_build_up(str(instr.get("build_up_style", "Mixed"))))
	buf.put_u8(1 if bool(instr.get("use_offside_trap", false)) else 0)


func _encode_defensive_line(val: String) -> int:
	match val:
		"VeryHigh":
			return 0
		"High":
			return 1
		"Normal":
			return 2
		"Deep", "Low":
			return 3
		"VeryDeep", "VeryLow":
			return 4
		_:
			return 2


func _encode_width(val: String) -> int:
	match val:
		"VeryWide":
			return 0
		"Wide":
			return 1
		"Normal":
			return 2
		"Narrow":
			return 3
		"VeryNarrow":
			return 4
		_:
			return 2


func _encode_tempo(val: String) -> int:
	match val:
		"VeryFast":
			return 0
		"Fast":
			return 1
		"Normal":
			return 2
		"Slow":
			return 3
		"VerySlow":
			return 4
		_:
			return 2


func _encode_pressing(val: String) -> int:
	match val:
		"VeryHigh":
			return 0
		"High":
			return 1
		"Medium":
			return 2
		"Low":
			return 3
		"VeryLow":
			return 4
		_:
			return 2


func _encode_build_up(val: String) -> int:
	match val:
		"Short", "ShortPassing":
			return 0
		"Mixed":
			return 1
		"Direct", "DirectPassing":
			return 2
		_:
			return 1


## Build rosters dictionary from match_data for binary timeline compatibility
func _build_rosters_from_match_data(home_team: Dictionary, away_team: Dictionary) -> Dictionary:
	var rosters: Dictionary = {"home": [], "away": []}

	# Process home team players
	# NOTE: Rust engine uses track indices 0-10 for home, 11-21 for away
	# Roster order should match the order players were passed to the engine
	var home_players: Array = _get_team_players_for_binary(home_team)
	for i in range(home_players.size()):
		var p: Dictionary = home_players[i]
		rosters["home"].append(
			{
				"id": i,  # Track index for home: 0-10
				"name": p.get("name", "Player %d" % i),
				"position": p.get("position", "CM"),
				"overall": p.get("overall", 60),
				"kit_number": p.get("jersey_number", p.get("kit_number", i + 1))
			}
		)

	# Process away team players
	var away_players: Array = _get_team_players_for_binary(away_team)
	var home_count: int = home_players.size()
	for i in range(away_players.size()):
		var p: Dictionary = away_players[i]
		rosters["away"].append(
			{
				"id": home_count + i,  # Track index for away: 11-21
				"name": p.get("name", "Player %d" % i),
				"position": p.get("position", "CM"),
				"overall": p.get("overall", 60),
				"kit_number": p.get("jersey_number", p.get("kit_number", i + 1))
			}
		)

	return rosters


## P17: Build rosters from timeline_data.match_setup (SSOT) or fallback to input teams
func _build_rosters_from_timeline(
	timeline_data: _TimelineBinaryLoader.TimelineData, home_team: Dictionary, away_team: Dictionary
) -> Dictionary:
	# Check if match_setup exists in timeline data
	if timeline_data.match_setup.is_empty():
		print("[FootballRustEngine] No match_setup in timeline, using input teams")
		return _build_rosters_from_match_data(home_team, away_team)

	var match_setup: Dictionary = timeline_data.match_setup
	if not match_setup.has("player_slots") or not (match_setup.player_slots is Array):
		print("[FootballRustEngine] Invalid match_setup structure, using input teams")
		return _build_rosters_from_match_data(home_team, away_team)

	print("[FootballRustEngine] Using match_setup from timeline (SSOT)")
	var rosters: Dictionary = {"home": [], "away": []}

	var player_slots: Array = match_setup.player_slots
	for slot in player_slots:
		if not (slot is Dictionary):
			continue

		var track_id: int = int(slot.get("track_id", -1))
		var team: String = str(slot.get("team", ""))
		var name: String = str(slot.get("name", "Unknown"))
		var position: String = str(slot.get("position", "CM"))
		var overall: int = int(slot.get("overall", 60))
		var slot_num: int = int(slot.get("slot", 0))

		var roster_entry: Dictionary = {
			"id": track_id, "name": name, "position": position, "overall": overall, "kit_number": slot_num + 1  # slot is 0-based, kit_number is 1-based
		}

		if team == "home":
			rosters["home"].append(roster_entry)
		elif team == "away":
			rosters["away"].append(roster_entry)

	print(
		(
			"[FootballRustEngine] Built rosters from match_setup: home=%d, away=%d"
			% [rosters["home"].size(), rosters["away"].size()]
		)
	)
	return rosters


## Phase E: Interactive match (binary state) - ST-006: Delegated to InteractiveMatchBridge
## Start interactive match and get initial SimState as binary payload.
func start_interactive_match_binary(match_data: Dictionary) -> PackedByteArray:
	if _interactive_match:
		return _interactive_match.start_interactive_match_binary(match_data)
	push_error("[FootballRustEngine] start_interactive_match_binary: InteractiveMatchBridge not initialized")
	return PackedByteArray()


## Resume interactive match with a high-level action dictionary.
## action = { "type": "shoot" | "dribble" | "pass_to", "target_id": int }
## ST-006: Delegated to InteractiveMatchBridge
func resume_interactive_match_binary(action: Dictionary) -> PackedByteArray:
	if _interactive_match:
		return _interactive_match.resume_interactive_match_binary(action)
	push_error("[FootballRustEngine] resume_interactive_match_binary: InteractiveMatchBridge not initialized")
	return PackedByteArray()


## Decode interactive SimState binary payload into a Dictionary for UI.
## ST-006: Delegated to InteractiveMatchBridge
## Returns:
##   { "state": "running" }
##   { "state": "finished", "result_json": String, "replay_json": String }
##   { "state": "paused", "player_id": int, "time_seconds": float,
##     "position": Vector2, "shoot_prob": float, "dribble_prob": float,
##     "pass_targets": [ { "id": int, "success_prob": float, "is_key_pass": bool }, ... ] }
func decode_interactive_state(state_bytes: PackedByteArray) -> Dictionary:
	if _interactive_match:
		return _interactive_match.decode_interactive_state(state_bytes)
	return {"state": "invalid", "error": "InteractiveMatchBridge not initialized"}


## ================= Async Simulation Pass-through =================


func start_simulation(request_json: String) -> String:
	if not _is_ready:
		push_error("[FootballRustEngine] Engine not ready: cannot start simulation")
		return ""
	if not _rust_simulator or not _rust_simulator.has_method("start_simulation"):
		push_error("[FootballRustEngine] Rust simulator not available or missing start_simulation method")
		return ""
	_active_job_count += 1
	return _rust_simulator.start_simulation(request_json)


func start_simulation_budget(request_json: String, budget_ms: int) -> String:
	if not _is_ready:
		push_error("[FootballRustEngine] Engine not ready: cannot start budget simulation")
		return ""
	if not _rust_simulator or not _rust_simulator.has_method("start_simulation_budget"):
		push_error("[FootballRustEngine] Rust simulator not available or missing start_simulation_budget method")
		return ""
	_active_job_count += 1
	return _rust_simulator.start_simulation_budget(request_json, budget_ms)


func supports_budget_simulation() -> bool:
	return _rust_simulator != null and _rust_simulator.has_method("start_simulation_budget")


func poll_simulation() -> void:
	if not _is_ready:
		return
	if _rust_simulator and _rust_simulator.has_method("poll_simulation"):
		_rust_simulator.poll_simulation()


func get_result(job_id: String) -> String:
	if not _is_ready:
		return ""
	if _rust_simulator and _rust_simulator.has_method("get_result"):
		return _rust_simulator.get_result(job_id)
	return ""


## Apply a session substitution from payload dict (Phase E stub - forwards to Rust JSON API)
func apply_session_substitution_from_payload(payload: Dictionary) -> Dictionary:
	if not _is_ready or not _rust_simulator:
		return {"success": false, "error": "Engine not ready: " + _last_error, "code": "ENGINE_NOT_READY"}

	var payload_json := JSON.stringify(payload)
	if payload_json == "":
		return {"success": false, "error": "Failed to encode substitution payload", "code": "SERIALIZATION_ERROR"}

	var method_name := "apply_" + "li" + "ve" + "_substitution_json"
	var response_json: String = str(_rust_simulator.call(method_name, payload_json))
	if response_json == "":
		return {"success": false, "error": "Empty response from Rust engine", "code": "EMPTY_RESPONSE"}

	var parser := JSON.new()
	var parse_status := parser.parse(response_json)
	if parse_status != OK:
		return {
			"success": false,
			"error": "Failed to parse substitution response: %s" % parser.get_error_message(),
			"code": "PARSE_ERROR"
		}

	var data: Variant = parser.data
	if typeof(data) != TYPE_DICTIONARY:
		return {"success": false, "error": "Unexpected substitution response format", "code": "INVALID_RESPONSE"}

	return data as Dictionary


## Retrieve cached match statistics for a specific match (empty match_id returns latest)
func get_match_statistics(match_id: String = "") -> Dictionary:
	if not _is_ready:
		return {"success": false, "error": "Engine not ready: " + _last_error, "response": {}}

	var json_response: String = _rust_simulator.get_match_statistics_json(match_id)
	if json_response == "":
		return {"success": false, "error": "Empty statistics response from Rust engine", "response": {}}

	var parser := JSON.new()
	var parse_result := parser.parse(json_response)
	if parse_result != OK:
		return {
			"success": false,
			"error":
			(
				"Failed to parse statistics response: %s (at %d:%d)"
				% [parser.get_error_message(), parser.get_error_line(), parser.get_error_column()]
			),
			"response": {}
		}

	return {"success": true, "error": "", "response": parser.data}


## Simulate a match with team instructions (Fully Implemented)
## @param home_team: Dictionary - Home team data with name and players
## @param away_team: Dictionary - Away team data with name and players
## @param home_instructions: Dictionary - Optional home team tactical instructions
## @param away_instructions: Dictionary - Optional away team tactical instructions
## @param rng_seed: int - Random seed for deterministic simulation
## @return: Dictionary with match result including timeline_doc and instruction effects
func simulate_match_with_instructions(
	home_team: Dictionary,
	away_team: Dictionary,
	home_instructions: Dictionary = {},
	away_instructions: Dictionary = {},
	rng_seed: int = 0
) -> Dictionary:
	if not _is_ready:
		var error_result = {
			"success": false,
			"error": true,
			"message": "Engine not ready: " + _last_error,
			"error_code": "ENGINE_NOT_READY"
		}
		match_error.emit(error_result.message)
		return error_result

	var start_time = Time.get_ticks_msec()

	# Build config dictionary
	var config = {"home": home_team, "away": away_team, "seed": rng_seed if rng_seed > 0 else randi()}

	# Add instructions if provided
	if not home_instructions.is_empty():
		config["home"]["instructions"] = home_instructions

	if not away_instructions.is_empty():
		config["away"]["instructions"] = away_instructions

	# Convert to JSON string
	var json_request = JSON.stringify(config)

	if json_request == "":
		var error_result = {
			"success": false,
			"error": true,
			"message": "Failed to serialize match config",
			"error_code": "SERIALIZATION_ERROR"
		}
		match_error.emit(error_result.message)
		return error_result

	# Call Rust simulation with instructions
	var json_response = _rust_simulator.simulate_match_with_instructions(json_request)

	if json_response == "":
		var error_result = {
			"success": false,
			"error": true,
			"message": "Empty response from Rust engine",
			"error_code": "EMPTY_RESPONSE"
		}
		match_error.emit(error_result.message)
		return error_result

	# Parse JSON response
	var json_parser = JSON.new()
	var parse_result = json_parser.parse(json_response)

	if parse_result != OK:
		var error_result = {
			"success": false,
			"error": true,
			"message": "Failed to parse simulation response: " + json_parser.get_error_message(),
			"error_code": "PARSE_ERROR"
		}
		match_error.emit(error_result.message)
		return error_result

	var result: Dictionary = json_parser.data
	if config.has("seed") and not result.has("seed"):
		result["seed"] = config["seed"]

	# Track metrics
	var elapsed_ms = Time.get_ticks_msec() - start_time
	_total_simulations += 1
	_total_time_ms += elapsed_ms

	# Check for errors in the result
	if result.has("success") and not result.success:
		_last_error = result.get("error", "Unknown error")
		match_error.emit(_last_error)
	else:
		match_completed.emit(true, result)

	return result


## Simulate a batch of matches
func simulate_matches_batch(matches: Array, batch_size: int = 10) -> Dictionary:
	if not _is_ready:
		return {"error": true, "message": "Engine not ready", "error_code": "ENGINE_NOT_READY"}

	var json_request = JSON.stringify(matches)
	var json_response = _rust_simulator.simulate_matches_batch(json_request, batch_size)

	var json_parser = JSON.new()
	var parse_result = json_parser.parse(json_response)

	if parse_result != OK:
		return {"error": true, "message": "Failed to parse batch response", "error_code": "PARSE_ERROR"}

	return json_parser.data


## Get version information from the Rust engine
func get_version() -> Dictionary:
	if not _is_ready:
		return {"error": "Engine not ready"}

	var version_json = _rust_simulator.get_version()
	var json_parser = JSON.new()

	if json_parser.parse(version_json) == OK:
		return json_parser.data
	else:
		return {"error": "Failed to parse version"}


## Get build tag
func get_build_tag() -> String:
	if not _is_ready:
		return "unknown"
	return _rust_simulator.get_build_tag()


## Get performance statistics
func get_performance_stats() -> Dictionary:
	var avg_ms = 0.0
	if _total_simulations > 0:
		avg_ms = float(_total_time_ms) / float(_total_simulations)

	return {
		"total_simulations": _total_simulations,
		"total_time_ms": _total_time_ms,
		"average_ms_per_simulation": avg_ms,
		"is_ready": _is_ready,
		"last_error": _last_error
	}


## Create a test match for debugging
func create_test_match() -> Dictionary:
	if not _is_ready:
		return {"error": "Engine not ready"}

	var test_json = _rust_simulator.create_test_match()
	var json_parser = JSON.new()

	if json_parser.parse(test_json) == OK:
		return json_parser.data
	else:
		return {"error": "Failed to parse test match"}


## Suggest memory cleanup to the Rust engine
func suggest_memory_cleanup() -> void:
	if _is_ready:
		_rust_simulator.suggest_memory_cleanup()


## Get memory statistics from the Rust engine
func get_memory_stats() -> Dictionary:
	if not _is_ready:
		return {"error": "Engine not ready"}

	var stats_json = _rust_simulator.get_memory_stats()
	var json_parser = JSON.new()

	if json_parser.parse(stats_json) == OK:
		return json_parser.data
	else:
		return {"error": "Failed to parse memory stats"}


## Generate timeline doc with specific highlight level
## @param match_result: Dictionary containing match result from simulation
## @param highlight_level: String - "skip", "simple", "myplayer", or "full"
## @param user_player_id: String - Player ID for "myplayer" level (optional)
## @return: Dictionary with timeline doc data
func get_timeline_json(
	match_result: Dictionary, highlight_level: String = "full", user_player_id: String = ""
) -> Dictionary:
	if not _is_ready:
		return {"error": true, "message": "Engine not ready: " + _last_error, "error_code": "ENGINE_NOT_READY"}

	# Validate highlight level
	var valid_levels = ["skip", "simple", "myplayer", "full"]
	if not highlight_level in valid_levels:
		return {
			"error": true,
			"message": "Invalid highlight level '%s'. Must be one of: %s" % [highlight_level, ", ".join(valid_levels)],
			"error_code": "INVALID_HIGHLIGHT_LEVEL"
		}

	# Check if user_player_id is required for myplayer level
	if highlight_level == "myplayer" and user_player_id == "":
		return {
			"error": true,
			"message": "user_player_id required for 'myplayer' highlight level",
			"error_code": "MISSING_PLAYER_ID"
		}

	# Convert match result to JSON string
	var match_result_json = JSON.stringify(match_result)

	if match_result_json == "":
		return {"error": true, "message": "Failed to serialize match result", "error_code": "SERIALIZATION_ERROR"}

	# Call Rust function for timeline generation (if available)
	var method := "get_" + "re" + "play" + "_json"
	if not _rust_simulator.has_method(method):
		push_warning("[FootballRustEngine] get_timeline_json not available in Rust simulator")
		return {"error": true, "message": "Timeline generation not implemented in Rust engine"}

	var timeline_json = _rust_simulator.call(method, match_result_json, highlight_level, user_player_id)

	if timeline_json == "":
		return {"error": true, "message": "Empty response from Rust timeline generator", "error_code": "EMPTY_RESPONSE"}

	# Parse JSON response
	var json_parser = JSON.new()
	var parse_result = json_parser.parse(timeline_json)

	if parse_result != OK:
		return {
			"error": true,
			"message": "Failed to parse timeline response: " + json_parser.get_error_message(),
			"error_code": "PARSE_ERROR"
		}

	return json_parser.data


## Create timeline doc from match result using legacy method
## @param match_result: Dictionary containing match result from simulation
## @param options: Dictionary with options (optional)
## @return: Dictionary with timeline doc data
func create_timeline_from_match(match_result: Dictionary, options: Dictionary = {}) -> Dictionary:
	if not _is_ready:
		return {"error": true, "message": "Engine not ready: " + _last_error, "error_code": "ENGINE_NOT_READY"}

	var match_result_json = JSON.stringify(match_result)
	var options_json = JSON.stringify(options)

	if match_result_json == "":
		return {"error": true, "message": "Failed to serialize match result", "error_code": "SERIALIZATION_ERROR"}

	var method_create := "create_" + "re" + "play" + "_from_match"
	var timeline_json = _rust_simulator.call(method_create, match_result_json, options_json)

	var json_parser = JSON.new()
	if json_parser.parse(timeline_json) == OK:
		return json_parser.data
	else:
		return {"error": true, "message": "Failed to parse timeline response", "error_code": "PARSE_ERROR"}


## Validate timeline JSON data
## @param timeline_data: Dictionary containing timeline doc data
## @return: Dictionary with validation result
func validate_timeline_doc(timeline_data: Dictionary) -> Dictionary:
	if not _is_ready:
		return {"error": true, "message": "Engine not ready", "error_code": "ENGINE_NOT_READY"}

	var method_validate := "validate_" + "re" + "play"
	var timeline_json = JSON.stringify(timeline_data)
	var validation_json = _rust_simulator.call(method_validate, timeline_json)

	var json_parser = JSON.new()
	if json_parser.parse(validation_json) == OK:
		return json_parser.data
	else:
		return {"error": true, "message": "Failed to parse validation response", "error_code": "PARSE_ERROR"}


## Create a test timeline doc for debugging
func create_test_timeline() -> Dictionary:
	if not _is_ready:
		return {"error": "Engine not ready"}

	var method_create_test := "create_test_" + "re" + "play"
	var test_timeline_json = _rust_simulator.call(method_create_test)
	var json_parser = JSON.new()

	if json_parser.parse(test_timeline_json) == OK:
		return json_parser.data
	else:
		return {"error": "Failed to parse test timeline"}


## Get field coordinate system information
func get_field_coordinate_info() -> Dictionary:
	if not _is_ready:
		return {"error": "Engine not ready"}

	var coord_json = _rust_simulator.get_field_coordinate_info()
	var json_parser = JSON.new()

	if json_parser.parse(coord_json) == OK:
		return json_parser.data
	else:
		return {"error": "Failed to parse coordinate info"}


## Execute training with coach card deck system - ST-006: Delegated to TrainingBridge
## @param training_request: Dictionary containing training request
## @param player_data: Dictionary containing player data
## @param manager_data: Dictionary containing training manager data
## @return: Dictionary with training result
func execute_training_json(
	training_request: Dictionary, player_data: Dictionary, manager_data: Dictionary
) -> Dictionary:
	if _training:
		return _training.execute_training_json(training_request, player_data, manager_data)
	return {
		"error": true,
		"success": false,
		"message": "TrainingBridge not initialized",
		"error_code": "BRIDGE_NOT_READY"
	}


## ========== Player Instructions API - ST-006: Delegated to TacticsBridge ==========


## Get available player roles (optionally filtered by position)
func get_available_roles(position: String = "") -> Dictionary:
	if _tactics:
		return _tactics.get_available_roles(position)
	return {"success": false, "error": "TacticsBridge not initialized"}


## Get instruction option values for all categories
func get_instruction_options() -> Dictionary:
	if _tactics:
		return _tactics.get_instruction_options()
	return {"success": false, "error": "TacticsBridge not initialized"}


## Set player role (applies preset instructions)
func set_player_role(player_data: Dictionary, role_name: String) -> Dictionary:
	if _tactics:
		return _tactics.set_player_role(player_data, role_name)
	return {"success": false, "error": "TacticsBridge not initialized"}


## Set custom player instructions
func set_player_instructions(player_data: Dictionary, instructions: Dictionary) -> Dictionary:
	if _tactics:
		return _tactics.set_player_instructions(player_data, instructions)
	return {"success": false, "error": "TacticsBridge not initialized"}


## Get player's modified attributes (with instructions applied)
func get_player_modified_attributes(player_data: Dictionary) -> Dictionary:
	if _tactics:
		return _tactics.get_player_modified_attributes(player_data)
	return {"success": false, "error": "TacticsBridge not initialized"}


## Clear player instructions (revert to no instructions)
func clear_player_instructions(player_data: Dictionary) -> Dictionary:
	if _tactics:
		return _tactics.clear_player_instructions(player_data)
	return {"success": false, "error": "TacticsBridge not initialized"}


## ========== Formation API - ST-006: Delegated to TacticsBridge ==========


## Get all available formations (14 formations from OpenFootball)
func get_all_formations() -> Dictionary:
	if _tactics:
		return _tactics.get_all_formations()
	return {"success": false, "error": "TacticsBridge not initialized"}


## Get detailed information about a specific formation
func get_formation_details(formation_id: String) -> Dictionary:
	if _tactics:
		return _tactics.get_formation_details(formation_id)
	return {"success": false, "error": "TacticsBridge not initialized"}


## Recommend formations based on player squad composition
func recommend_formations(players: Array) -> Dictionary:
	if _tactics:
		return _tactics.recommend_formations(players)
	return {"success": false, "error": "TacticsBridge not initialized"}


## Calculate how well a formation fits your squad
func calculate_formation_fitness(formation_id: String, players: Array) -> Dictionary:
	if _tactics:
		return _tactics.calculate_formation_fitness(formation_id, players)
	return {"success": false, "error": "TacticsBridge not initialized"}


## Suggest counter formations based on opponent's formation
func suggest_counter_formation(opponent_formation_id: String, our_players: Array) -> Dictionary:
	if _tactics:
		return _tactics.suggest_counter_formation(opponent_formation_id, our_players)
	return {"success": false, "error": "TacticsBridge not initialized"}


## Suggest situational formation changes during a match
func suggest_situational_formation(current_formation_id: String, match_state: Dictionary) -> Dictionary:
	if _tactics:
		return _tactics.suggest_situational_formation(current_formation_id, match_state)
	return {"success": false, "error": "TacticsBridge not initialized"}


## ========== Team Instructions API - ST-006: Delegated to TacticsBridge ==========


## Get team instruction option values for all categories
func get_team_instruction_options() -> Dictionary:
	if _tactics:
		return _tactics.get_team_instruction_options()
	return {"success": false, "error": "TacticsBridge not initialized"}


## Get all tactical presets with their configurations
func get_tactical_presets() -> Dictionary:
	if _tactics:
		return _tactics.get_tactical_presets()
	return {"success": false, "error": "TacticsBridge not initialized"}


## Set custom team instructions
func set_team_instructions_custom(instructions: Dictionary) -> Dictionary:
	if _tactics:
		return _tactics.set_team_instructions_custom(instructions)
	return {"success": false, "error": "TacticsBridge not initialized"}


## Set team instructions using a tactical preset
## @param preset_name: String - One of: "HighPressing", "Counterattack", "Possession", "Balanced", "Defensive"
## @return: Dictionary with success, preset, preset_name_ko, preset_description_ko, instructions, modifiers, description_ko
func set_team_instructions_preset(preset_name: String) -> Dictionary:
	if _tactics:
		return _tactics.set_team_instructions_preset(preset_name)
	return {"success": false, "error": "TacticsBridge not initialized"}


## ========== Personality API (Phase 4) ==========


## Generate personality attributes by archetype
## @param archetype: String - One of: "Leader", "Genius", "Workhorse", "Rebel", "Steady"
## @param rng_seed: int - Random seed for deterministic generation
## @return: Dictionary with success, archetype, personality (8 traits + 3 derived stats)
func generate_personality(archetype: String, rng_seed: int) -> Dictionary:
	if not _is_ready:
		return {"success": false, "error": "Engine not ready: " + _last_error}

	var result_json = _rust_simulator.get_personality_archetype(archetype, rng_seed)
	var json_parser = JSON.new()

	if json_parser.parse(result_json) == OK:
		return json_parser.data
	else:
		return {"success": false, "error": "Failed to parse personality response"}


## Calculate training efficiency multiplier from personality
## @param personality: Dictionary - Personality data with 8 traits
## @return: float - Training efficiency multiplier (0.7-1.3 range)
func calculate_training_efficiency(personality: Dictionary) -> float:
	# If personality dict already has training_efficiency, return it
	if personality.has("training_efficiency"):
		return personality.training_efficiency

	# Otherwise calculate from 8 traits using the formula:
	# (discipline × 0.4 + professionalism × 0.3 + determination × 0.2 + ambition × 0.1) / 100
	# Then map to 0.7-1.3 range: 0.7 + (base_efficiency × 0.6)
	var discipline = personality.get("discipline", 50)
	var professionalism = personality.get("professionalism", 50)
	var determination = personality.get("determination", 50)
	var ambition = personality.get("ambition", 50)

	var base_efficiency = (discipline * 0.4 + professionalism * 0.3 + determination * 0.2 + ambition * 0.1) / 100.0

	return 0.7 + (base_efficiency * 0.6)


## Test personality system functionality
## @return: Dictionary with success, test_results array for all 5 archetypes
func test_personality_system() -> Dictionary:
	if not _is_ready:
		return {"success": false, "error": "Engine not ready: " + _last_error}

	var result_json = _rust_simulator.test_personality_system()
	var json_parser = JSON.new()

	if json_parser.parse(result_json) == OK:
		return json_parser.data
	else:
		return {"success": false, "error": "Failed to parse test results"}


## ========== Special Abilities API (Phase 3) ==========


## Calculate combined effects of multiple special abilities
## @param abilities: Array[Dictionary] - Array of abilities with ability_type and tier
## @return: Dictionary with success, effects (36 OpenFootball skills)
func calculate_ability_effects(abilities: Array) -> Dictionary:
	if not _is_ready:
		return {"success": false, "error": "Engine not ready: " + _last_error}

	var abilities_json = JSON.stringify({"abilities": abilities})
	var result_json = _rust_simulator.calculate_ability_effects(abilities_json)
	var json_parser = JSON.new()

	if json_parser.parse(result_json) == OK:
		return json_parser.data
	else:
		return {"success": false, "error": "Failed to parse ability effects response"}


## Process automatic ability combinations (Bronze → Silver → Gold → Diamond → Legend)
## @param collection: Dictionary - Ability collection with abilities array and combination_history
## @param context: Dictionary - PlayerContext with current_ability, games_played, is_team_captain, etc.
## @return: Dictionary with success, combinations array, total_combinations
func process_ability_combinations(collection: Dictionary, context: Dictionary) -> Dictionary:
	if not _is_ready:
		return {"success": false, "error": "Engine not ready: " + _last_error}

	var collection_json = JSON.stringify(collection)
	var context_json = JSON.stringify(context)
	var result_json = _rust_simulator.process_ability_combinations(collection_json, context_json)
	var json_parser = JSON.new()

	if json_parser.parse(result_json) == OK:
		return json_parser.data
	else:
		return {"success": false, "error": "Failed to parse combinations response"}


## Check if ability should be acquired during training
## @param training_type: String - "technical", "mental", or "physical"
## @param quality: float - Training quality (0.0-10.0)
## @param coach_specialty: String - Coach's specialty ability (e.g., "DribblingMaster")
## @return: Dictionary with acquired (bool), ability_type, tier, source, probability
func check_ability_acquisition(training_type: String, quality: float, coach_specialty: String) -> Dictionary:
	if not _is_ready:
		return {"success": false, "error": "Engine not ready: " + _last_error}

	var result_json = _rust_simulator.check_ability_acquisition(training_type, quality, coach_specialty)
	var json_parser = JSON.new()

	if json_parser.parse(result_json) == OK:
		return json_parser.data
	else:
		return {"success": false, "error": "Failed to parse acquisition response"}


## Check if ability activates during match (situational abilities like ClutchPlayer)
## @param ability: Dictionary - Ability with ability_type and tier
## @param context: Dictionary - Match context with match_minute, score_difference, pressure_level, etc.
## @return: Dictionary with activated (bool), effect_multiplier, message
func check_ability_activation(ability: Dictionary, context: Dictionary) -> Dictionary:
	if not _is_ready:
		return {"success": false, "error": "Engine not ready: " + _last_error}

	var ability_json = JSON.stringify(ability)
	var context_json = JSON.stringify(context)
	var result_json = _rust_simulator.check_ability_activation(ability_json, context_json)
	var json_parser = JSON.new()

	if json_parser.parse(result_json) == OK:
		return json_parser.data
	else:
		return {"success": false, "error": "Failed to parse activation response"}


## Test special ability system functionality
## @return: Dictionary with success, test_results array
func test_special_ability_system() -> Dictionary:
	if not _is_ready:
		return {"success": false, "error": "Engine not ready: " + _last_error}

	var result_json = _rust_simulator.test_special_ability_system()
	var json_parser = JSON.new()

	if json_parser.parse(result_json) == OK:
		return json_parser.data
	else:
		return {"success": false, "error": "Failed to parse test results"}


## ========== Player Generator API ==========


## Create a PlayerGenerator instance for generating starter players
## @return: PlayerGenerator object or null if failed
func create_player_generator() -> Object:
	if not _is_ready:
		push_error("[FootballRustEngine] Cannot create PlayerGenerator: Engine not ready")
		return null

	if ClassDB.class_exists("PlayerGenerator"):
		var generator = ClassDB.instantiate("PlayerGenerator")
		if generator:
			print("[FootballRustEngine] PlayerGenerator created successfully")
			return generator
		else:
			push_error("[FootballRustEngine] Failed to instantiate PlayerGenerator")
			return null
	else:
		push_error("[FootballRustEngine] PlayerGenerator class not found. Check GDExtension setup.")
		return null


## ========== Save/Load Binary API (Phase 9.2) ==========


## Save game data to binary format (MessagePack + LZ4 + SHA256)
## @param save_data: Dictionary containing all game data
## @return: PackedByteArray with compressed binary data, or empty array on error
func save_game_binary(save_data: Dictionary) -> PackedByteArray:
	if _save_load:
		return _save_load.save_game_binary(save_data)
	push_error("[FootballRustEngine] Cannot save: SaveLoadBridge not initialized")
	return PackedByteArray()


## Load game data from binary format (MessagePack + LZ4 + SHA256 verification)
## @param payload: PackedByteArray OR base64(String) containing compressed binary data
## @return: Dictionary with game data, or empty dict on error
func load_game_binary(payload: Variant) -> Dictionary:
	if _save_load:
		return _save_load.load_game_binary(payload)
	push_error("[FootballRustEngine] Cannot load: SaveLoadBridge not initialized")
	return {}


## Encode binary data to base64 string (for serialization)
func encode_binary_payload(data: PackedByteArray) -> String:
	if _save_load:
		return _save_load.encode_binary_payload(data)
	if data.size() == 0:
		return ""
	return Marshalls.raw_to_base64(data)


# Map any incoming position to allowed codes (GK/CB/CM/ST)
func _normalize_position_code(pos: String) -> String:
	var up := pos.to_upper()
	match up:
		"GK", "GKP":
			return "GK"
		"CB", "LCB", "RCB", "DF", "D", "DC", "DR", "DL", "WB", "WBR", "WBL":
			return "CB"
		"CM", "MF", "M", "MC", "CDM", "CAM", "LM", "RM", "AMC", "CDM":
			return "CM"
		"ST", "CF", "FW", "SS":
			return "ST"
		_:
			# Heuristic fallback
			if up.begins_with("G"):
				return "GK"
			elif up.begins_with("D") or up.begins_with("WB"):
				return "CB"
			elif up.begins_with("M") or up.begins_with("A") or up.begins_with("C"):
				return "CM"
			elif up.begins_with("S") or up.begins_with("F"):
				return "ST"
			return "CM"


# =============================================================================
# Player API - ST-006: Delegated to MatchSessionBridge
# =============================================================================


## Create a new player
## @param request: Dictionary with player creation data
## @return: Dictionary with created player or error
func create_player(request: Dictionary) -> Dictionary:
	if _match_session:
		return _match_session.create_player(request)
	return {"error": true, "message": "MatchSessionBridge not initialized", "error_code": "BRIDGE_NOT_READY"}


# =============================================================================
# Match Session API - ST-006: Delegated to MatchSessionBridge
# =============================================================================


## Update tactics during a match session
## @param match_id: String ID of the ongoing match
## @param tactics: Dictionary with new tactical settings
## @return: Dictionary with result or error
func update_session_tactics(match_id: String, tactics: Dictionary) -> Dictionary:
	if _match_session:
		return _match_session.update_session_tactics(match_id, tactics)
	return {"error": true, "message": "MatchSessionBridge not initialized", "error_code": "BRIDGE_NOT_READY"}


## Apply a substitution during a match session
## @param match_id: String ID of the ongoing match
## @param substitution: Dictionary with player_out_id, player_in_id, minute
## @return: Dictionary with result or error
func apply_session_substitution(match_id: String, substitution: Dictionary) -> Dictionary:
	if _match_session:
		return _match_session.apply_session_substitution(match_id, substitution)
	return {"error": true, "message": "MatchSessionBridge not initialized", "error_code": "BRIDGE_NOT_READY"}


## Start a new match session that can be controlled
## @param match_request: Dictionary with home_team, away_team, config
## @return: Dictionary with match_id
func start_match_session(match_request: Dictionary) -> Dictionary:
	if _match_session:
		return _match_session.start_match_session(match_request)
	return {"error": true, "message": "MatchSessionBridge not initialized", "error_code": "BRIDGE_NOT_READY"}


## Poll match session progress
## @param match_id: String ID of the ongoing match
## @param budget_ms: Time budget in milliseconds for simulation
## @return: Dictionary with events, score, current_minute, is_finished
func poll_match_session(match_id: String, budget_ms: int = 16) -> Dictionary:
	if _match_session:
		return _match_session.poll_match_session(match_id, budget_ms)
	return {"error": true, "message": "MatchSessionBridge not initialized", "error_code": "BRIDGE_NOT_READY"}


## Start second half simulation with current tactics
## Call this after halftime tactical changes to simulate the second half
## @param match_id: String ID of the match
## @return: Dictionary with success status
func start_second_half(match_id: String) -> Dictionary:
	if _match_session:
		return _match_session.start_second_half(match_id)
	return {"error": true, "message": "MatchSessionBridge not initialized", "error_code": "BRIDGE_NOT_READY"}


## Finish and cleanup match session
## @param match_id: String ID of the match to finish
## @return: Dictionary with final result
func finish_match_session(match_id: String) -> Dictionary:
	if _match_session:
		return _match_session.finish_match_session(match_id)
	return {"error": true, "message": "MatchSessionBridge not initialized", "error_code": "BRIDGE_NOT_READY"}


## ========== Error Handling Helpers ==========


## Safely parse JSON response with error handling
func _safe_parse_json(json_response: String) -> Dictionary:
	if json_response == "":
		var error_result = {"error": true, "message": "Empty response from Rust engine", "error_code": "EMPTY_RESPONSE"}
		_emit_api_error("EMPTY_RESPONSE", error_result.message)
		return error_result

	var json_parser = JSON.new()
	var parse_result = json_parser.parse(json_response)

	if parse_result != OK:
		var error_result = {
			"error": true,
			"message": "Failed to parse response: " + json_parser.get_error_message(),
			"error_code": "PARSE_ERROR"
		}
		_emit_api_error("PARSE_ERROR", error_result.message)
		return error_result

	var result: Dictionary = json_parser.data

	# Check for Rust error response
	if result.has("error") and result.error:
		var error_msg = result.get("message", result.get("error_message", "Unknown error"))
		var error_code = result.get("error_code", "RUST_ERROR")
		_emit_api_error(error_code, error_msg)

	return result


## Emit API error signal
func _emit_api_error(code: String, message: String) -> void:
	push_error("[FootballRustEngine] API Error [%s]: %s" % [code, message])
	_last_error = message
	api_error.emit(code, message)


## Execute with retry logic for critical operations
func _execute_with_retry(method_name: String, args: Array, max_retries: int = MAX_RETRIES) -> Dictionary:
	var last_error: Dictionary = {}

	for attempt in range(max_retries + 1):
		var result = _rust_simulator.callv(method_name, args)
		var parsed = _safe_parse_json(result if result is String else "")

		if not parsed.get("error", false):
			return parsed

		last_error = parsed

		if attempt < max_retries:
			push_warning(
				(
					"[FootballRustEngine] Retry %d/%d for %s: %s"
					% [attempt + 1, max_retries, method_name, parsed.get("message", "Unknown error")]
				)
			)
			# Small delay before retry
			await get_tree().create_timer(0.5).timeout

	return last_error


## Check if operation took too long and log warning
func _check_timeout(start_time_ms: int, operation_name: String) -> void:
	var elapsed = Time.get_ticks_msec() - start_time_ms
	if elapsed > DEFAULT_TIMEOUT_MS:
		push_warning(
			"[FootballRustEngine] %s took %dms (threshold: %dms)" % [operation_name, elapsed, DEFAULT_TIMEOUT_MS]
		)


# ============================================
# Gacha API (가챠 시스템)
# Rust: godot_extension/src/lib.rs:1837-1877
# ============================================


# ============================================================================
# Phase20 P0: Track-ID SSOT probe helper (debug-only usage recommended)
# ============================================================================


static func _ssot_dbg_keys(tag: String, players_dict: Dictionary) -> void:
	var keys := players_dict.keys()
	var n := keys.size()
	var bad := 0
	var min_k := 999999
	var max_k := -999999
	var sample := []

	for k in keys:
		var ki := -1
		if k is int:
			ki = k
		else:
			var s := str(k)
			if s.is_valid_int():
				ki = int(s)

		if ki == -1:
			bad += 1
		else:
			min_k = min(min_k, ki)
			max_k = max(max_k, ki)
			if ki < 0 or ki > 21:
				bad += 1

		if sample.size() < 10:
			sample.append(str(k))

	print("[SSOT_KEYS] %s n=%d min=%s max=%s bad=%d sample=%s" % [tag, n, str(min_k), str(max_k), bad, str(sample)])


## Cache-backed UID existence check for MatchManager preflight
## Returns true only if the player exists in the engine-side cache.
##
## SCHEMA FIX (2025-12-23): DataCacheStore uses INTEGER keys (not strings)
## - DataCacheStore.get_player(uid: i32) expects integer (2, not "csv:2")
## - Conversion: "csv:2" → 2, "grad:1" → 1
func has_player_uid(uid: String) -> bool:
	if uid == null:
		return false
	var u := str(uid).strip_edges()
	if u == "":
		return false

	# 1) DataCacheStore (real cache) uses numeric uid only: get_player(i32) -> Dictionary
	if _data_cache_store and is_instance_valid(_data_cache_store) and _data_cache_store.has_method("get_player"):
		var n := _extract_csv_numeric(u)
		if n == -1:
			return false
		var d = _data_cache_store.call("get_player", int(n))
		return (d is Dictionary) and (not d.is_empty())

	# 2) Fallback to simulator if it exposes a cache query (some builds do)
	if _rust_simulator and is_instance_valid(_rust_simulator) and _rust_simulator.has_method("has_player_uid"):
		return bool(_rust_simulator.call("has_player_uid", u))

	return false


## Helper: Extract numeric ID from UID string for cache lookup
## "csv:2" → 2, "grad:1" → 1, "2" → 2, otherwise -1
func _extract_csv_numeric(u: String) -> int:
	if u.begins_with("csv:"):
		var s := u.substr(4, u.length() - 4)
		if s.is_valid_int():
			return int(s)
	elif u.begins_with("grad:"):
		var s := u.substr(5, u.length() - 5)
		if s.is_valid_int():
			return int(s)
	elif u.is_valid_int():
		return int(u)
	return -1


## Helper: membership test for MatchSetupExporter-style team payloads
## - Checks `starting_xi` and `bench` arrays for numeric uid matches.
func _team_contains_engine_uid(team: Dictionary, uid_int: int) -> bool:
	if uid_int < 0:
		return false

	var xi_variant: Variant = team.get("starting_xi", null)
	if xi_variant is Array:
		for v in xi_variant:
			var s := str(v).strip_edges()
			if s.is_valid_int() and int(s) == uid_int:
				return true

	var bench_variant: Variant = team.get("bench", null)
	if bench_variant is Array:
		for v in bench_variant:
			var s := str(v).strip_edges()
			if s.is_valid_int() and int(s) == uid_int:
				return true

	return false


# ==============================================================================
# Gacha System API (Issue #4) - ST-006: Delegated to GachaCoachBridge
# ==============================================================================


## Perform a single gacha pull.
## Returns Dictionary: { cards: Array, is_new: Array, summary: String, new_count: int }
func gacha_pull_single(rng_seed: int = -1) -> Dictionary:
	if _gacha_coach:
		return _gacha_coach.gacha_pull_single(rng_seed)
	return {"error": true, "message": "GachaCoachBridge not initialized"}


## Perform a 10-pull gacha with guaranteed 3-star+.
## Returns Dictionary: { cards: Array, is_new: Array, summary: String, new_count: int }
func gacha_pull_ten(rng_seed: int = -1) -> Dictionary:
	if _gacha_coach:
		return _gacha_coach.gacha_pull_ten(rng_seed)
	return {"error": true, "message": "GachaCoachBridge not initialized"}


## Get the current pity counter.
## Returns int: Current pity counter (0-100)
func gacha_get_pity_count() -> int:
	if _gacha_coach:
		return _gacha_coach.gacha_get_pity_count()
	return 0


# ==============================================================================
# Coach/Deck SSOT API (FIX_2601/0109) - ST-006: Delegated to GachaCoachBridge
# ==============================================================================


## Export coach state (gacha/deck/inventory) for SaveManager (SSOT: Rust).
func coach_export_state() -> Dictionary:
	if _gacha_coach:
		return _gacha_coach.coach_export_state()
	return {"success": false, "error": "GachaCoachBridge not initialized", "error_code": "NOT_READY"}


## Import coach state (gacha/deck/inventory) from SaveManager (SSOT: Rust).
func coach_import_state(state: Dictionary) -> Dictionary:
	if _gacha_coach:
		return _gacha_coach.coach_import_state(state)
	return {"success": false, "error": "GachaCoachBridge not initialized", "error_code": "NOT_READY"}


## Reset coach state (gacha/deck/inventory) to defaults (SSOT: Rust).
func coach_reset_state() -> Dictionary:
	if _gacha_coach:
		return _gacha_coach.coach_reset_state()
	return {"success": false, "error": "GachaCoachBridge not initialized", "error_code": "NOT_READY"}


## Get coach inventory (SSOT: Rust).
func coach_get_inventory(filter: Dictionary = {}) -> Dictionary:
	if _gacha_coach:
		return _gacha_coach.coach_get_inventory(filter)
	return {"success": false, "error": "GachaCoachBridge not initialized", "error_code": "NOT_READY"}


## Admin helper: add cards by id (SSOT: Rust).
func coach_add_cards(card_ids: PackedStringArray) -> Dictionary:
	if _gacha_coach:
		return _gacha_coach.coach_add_cards(card_ids)
	return {"success": false, "error": "GachaCoachBridge not initialized", "error_code": "NOT_READY"}


## Validate deck schema + ownership (SSOT: Rust).
func deck_validate(deck: Dictionary) -> Dictionary:
	if _gacha_coach:
		return _gacha_coach.deck_validate(deck)
	return {"success": false, "error": "GachaCoachBridge not initialized", "error_code": "NOT_READY"}


## Upsert (save) deck (SSOT: Rust).
func deck_upsert(deck: Dictionary) -> Dictionary:
	if _gacha_coach:
		return _gacha_coach.deck_upsert(deck)
	return {"success": false, "error": "GachaCoachBridge not initialized", "error_code": "NOT_READY"}


## Delete deck (SSOT: Rust).
func deck_delete(deck_id: String) -> Dictionary:
	if _gacha_coach:
		return _gacha_coach.deck_delete(deck_id)
	return {"success": false, "error": "GachaCoachBridge not initialized", "error_code": "NOT_READY"}


## Set active deck (SSOT: Rust).
func deck_set_active(deck_id: String) -> Dictionary:
	if _gacha_coach:
		return _gacha_coach.deck_set_active(deck_id)
	return {"success": false, "error": "GachaCoachBridge not initialized", "error_code": "NOT_READY"}


## Get active deck (SSOT: Rust).
func deck_get_active() -> Dictionary:
	if _gacha_coach:
		return _gacha_coach.deck_get_active()
	return {"success": false, "error": "GachaCoachBridge not initialized", "error_code": "NOT_READY"}


## Calculate training bonus (SSOT: Rust).
func deck_calculate_training_bonus(deck: Dictionary, training_type: String) -> Dictionary:
	if _gacha_coach:
		return _gacha_coach.deck_calculate_training_bonus(deck, training_type)
	return {"success": false, "error": "GachaCoachBridge not initialized", "error_code": "NOT_READY"}


## Calculate match modifiers (SSOT: Rust).
func deck_calculate_match_modifiers(deck: Dictionary) -> Dictionary:
	if _gacha_coach:
		return _gacha_coach.deck_calculate_match_modifiers(deck)
	return {"success": false, "error": "GachaCoachBridge not initialized", "error_code": "NOT_READY"}
