extends Node
## ============================================================================
## MatchSimulationManager - Match Simulation Execution
## ============================================================================
##
## PURPOSE: Match scheduling, simulation execution, and result processing
##
## SSOT RESPONSIBILITIES:
## - Simulation execution (engine interaction)
## - Position data during/after simulation
## - Timeline/replay data generation
##
## DATA FLOW (ST-002 SSOT):
## - Owns simulation execution via FootballRustEngine
## - Forwards complete results to MatchManager.ingest_external_match()
## - position_data always included in payload (no fallback needed in MatchManager)
## - Unidirectional: MatchSimulationManager → MatchManager (no reverse dependency)
##
## RELATED:
## - MatchManager: Receives results, manages state/history
## - FootballRustEngine: Engine interface for simulation
## ============================================================================
## Phase 6: Match Simulation Manager
## Handles match scheduling, simulation, and result processing
## 15 matches over 3 years (31% of total turns)

# Preload scripts to avoid class_name resolution issues
const _InteractiveMatchController := preload("res://autoload/domain/InteractiveMatchController.gd")
const _PlayerAppearanceBridge := preload("res://scripts/character/player_appearance_bridge.gd")
const _TeamUniformManager := preload("res://scripts/character/team_uniform_manager.gd")
const _MatchSessionController := preload("res://scripts/match_pipeline/MatchSessionController.gd")
const _DebugSettings := preload("res://scripts/config/debug_settings.gd")
const PositionSnapshotAdapter := preload("res://scripts/match_pipeline/PositionSnapshotAdapter.gd")
const _MatchSetup = preload("res://scripts/core/MatchSetup.gd")
const _PlayerLibrary = preload("res://scripts/core/PlayerLibrary.gd")
const _MatchSetupBuilder = preload("res://scripts/core/MatchSetupBuilder.gd")
const _InteractiveMatchSetupBuilder = preload("res://scripts/core/InteractiveMatchSetupBuilder.gd")
const _MatchPlayer = preload("res://scripts/core/MatchPlayer.gd")
const _PlayerStatsProcessor = preload("res://scripts/core/PlayerStatsProcessor.gd")
const _XpProcessor = preload("res://scripts/core/XpProcessor.gd")

# ST-005 Phase 1: Extracted components
const EventParser = preload("res://autoload/domain/match_simulation/EventParser.gd")
const MVPCareerHelper = preload("res://autoload/domain/match_simulation/MVPCareerHelper.gd")
# ST-005 Phase 2: Extracted components
const TacticsManager = preload("res://autoload/domain/match_simulation/TacticsManager.gd")
const RosterBuilder = preload("res://autoload/domain/match_simulation/RosterBuilder.gd")
# ST-005 Phase 3: Extracted components
const UserCommandDispatcher = preload("res://autoload/domain/match_simulation/UserCommandDispatcher.gd")
const MatchScheduler = preload("res://autoload/domain/match_simulation/MatchScheduler.gd")

# NOTE: Legacy SkeletonPlayer/Skeleton2DPlayer renderers removed (2025-12-09)
# Now using Socceralia sprites via HorizontalMatchViewer/SoccerPlayer

# Phase20: UnifiedFramePipeline integration (2025-12-18)
# - Wires MatchSessionController tick stream -> UnifiedFramePipeline.push_tick()
# - Viewer connects to UnifiedFramePipeline (not MatchSimulationManager)
# - Spec: docs/specs/spec_v5/fix/phase20/PHASE20_UNIFIED_FRAME_PIPELINE_SPEC.md

# ============================================================================
# SIGNALS
# ============================================================================

signal match_scheduled(match_data: Dictionary)
signal match_started(match_data: Dictionary)
signal match_completed(success: bool, result: Dictionary)
signal match_preparation_required(match_data: Dictionary)
signal session_event_appended(event: Dictionary)
signal match_simulation_finished(result: Dictionary)  # NEW: Emitted after match simulation is complete

# Phase 9: Match session streaming signals
signal match_state_updated(state: Dictionary)  # Time, score, period updates

# Phase E: Interactive mode signals
signal intervention_requested(context: Dictionary)  # Emitted when user intervention is needed
signal interactive_match_finished(result: Dictionary)  # Emitted when interactive match ends

# ============================================================================
# CONSTANTS
# ============================================================================

## Field dimensions (meters) - standard football pitch
const FIELD_LENGTH := 105.0
const FIELD_WIDTH := 68.0

## TeamView observation (debug)
@export var enable_team_view_observation: bool = false
@export var team_view_observer_is_home: bool = true

## Phase 23 (Career): StageManager league_config.json is the SSOT for league opponents.
## - League opponents + CA ranges derive from StageManager.active_league_id
## - DivisionManager remains legacy for table/progression (migration tracked in specs)
## League match weeks: 8, 12, 20, 28, 36, 44 (6 total per season)
## Cup matches (4 per season) are managed by CupManager (weeks 16, 24, 32, 40)
const CAREER_LEAGUE_MATCH_WEEKS := [8, 12, 20, 28, 36, 44]
const CAREER_LEAGUE_OPPONENT_SOURCE := "stage_league"

const MVP_MATCH_SCHEDULE = [
	{
		"week": 1,
		"type": "friendly",
		"opponent": "Academy Unity",
		"importance": 4,
		"overall_rating": 62,
		"seed": 744120001
	},
	{"week": 2, "type": "league", "opponent": "Rising Stars", "importance": 5, "overall_rating": 64, "seed": 744120002},
	{"week": 3, "type": "league", "opponent": "North Ridge", "importance": 5, "overall_rating": 65, "seed": 744120003},
	{
		"week": 4,
		"type": "league",
		"opponent": "Metro Juniors",
		"importance": 6,
		"overall_rating": 66,
		"seed": 744120004
	},
	{"week": 5, "type": "cup", "opponent": "Cup Challengers", "importance": 6, "overall_rating": 67, "seed": 744120005},
	{"week": 6, "type": "league", "opponent": "East Academy", "importance": 6, "overall_rating": 68, "seed": 744120006},
	{"week": 7, "type": "league", "opponent": "South City", "importance": 6, "overall_rating": 69, "seed": 744120007},
	{"week": 8, "type": "league", "opponent": "Westfield", "importance": 7, "overall_rating": 70, "seed": 744120008},
	{"week": 9, "type": "league", "opponent": "Capstone FC", "importance": 7, "overall_rating": 72, "seed": 744120009},
	{"week": 10, "type": "cup", "opponent": "District Elite", "importance": 7, "overall_rating": 74, "seed": 744120010},
	{
		"week": 11,
		"type": "league",
		"opponent": "Highland Youth",
		"importance": 8,
		"overall_rating": 76,
		"seed": 744120011
	},
	{
		"week": 12,
		"type": "league",
		"opponent": "Elite Juniors",
		"importance": 8,
		"overall_rating": 78,
		"seed": 744120012
	}
]

const TEAM_TRAINING_BY_MATCH_TYPE = {
	"friendly": "physical",
	"cup": "tactical",
	"league": "tactical",
	"national": "defending",
	"national_qualifier": "defending"
}

const OPPONENT_CSV_PATH := "res://data/players_with_pseudonym.csv"


func _use_real_names_for_opponents() -> bool:
	if OS.has_environment("OPPONENT_USE_REAL_NAMES"):
		var value := OS.get_environment("OPPONENT_USE_REAL_NAMES").strip_edges().to_lower()
		if value in ["0", "false", "no"]:
			return false
		if value in ["1", "true", "yes"]:
			return true
	return true


## Opponent CA ranges by match type
const OPPONENT_CA_RANGES = {
	"league": {1: [60, 90], 2: [90, 115], 3: [115, 140]},  # U16  # U17  # U18
	"cup": {"base_offset": [-10, 10]},
	"national_qualifier": {"base_offset": [0, 20]},
	"national": {"base_offset": [10, 30]}
}

## Match result fatigue constants
const FATIGUE_BASE = 35
const FATIGUE_EXTRA_TIME = 15

## Default tactical instructions
const DEFAULT_INSTRUCTIONS = {
	"attacking_intensity": 50,
	"defensive_line": 50,
	"pressing": 50,
	"tempo": 50,
	"width": 50,
	"passing_style": 50
}

# =============================================================================
# Format Conversion Helpers (ST-005 Phase 2 - Delegated)
# =============================================================================
# Implementation moved to TacticsManager.gd

func _convert_to_engine_format(formation: String) -> String:
	if _tactics_manager:
		return _tactics_manager.convert_to_engine_format(formation)
	return "T" + formation.replace("-", "")


func _convert_from_engine_format(formation_id: String) -> String:
	if _tactics_manager:
		return _tactics_manager.convert_from_engine_format(formation_id)
	return formation_id


func _normalize_stage_formation_to_display(formation_id: String) -> String:
	if _tactics_manager:
		return _tactics_manager.normalize_stage_formation_to_display(formation_id)
	var raw := formation_id.strip_edges()
	if raw == "":
		return "4-4-2"
	if raw.find("-") != -1:
		return raw
	return _convert_from_engine_format(raw)


func _ensure_player_instructions_from_team_tactics() -> void:
	if not player_instructions.is_empty() and _player_instructions_source != "team_tactics":
		if OS.is_debug_build():
			print("[MatchSimulationManager] Skip team tactics seed (source=%s)" % _player_instructions_source)
		return
	var derived := _build_instructions_from_team_tactics()
	if derived.is_empty():
		return
	player_instructions = derived
	_player_instructions_source = "team_tactics"
	print("[MatchSimulationManager] 📌 Applied team tactics as default instructions")


func _build_instructions_from_team_tactics() -> Dictionary:
	if not MyTeamData or not MyTeamData.has_method("get_team_tactics"):
		return {}
	var tactics: Variant = MyTeamData.get_team_tactics()
	if tactics is Dictionary:
		var params: Variant = (tactics as Dictionary).get("parameters", {})
		if params is Dictionary and not (params as Dictionary).is_empty():
			var p := params as Dictionary
			var instructions := DEFAULT_INSTRUCTIONS.duplicate()
			instructions["attacking_intensity"] = int(
				round(clampf(float(p.get("attacking_intensity", 0.5)) * 100.0, 0.0, 100.0))
			)
			instructions["defensive_line"] = int(
				round(clampf(float(p.get("defensive_line_height", 0.5)) * 100.0, 0.0, 100.0))
			)
			instructions["pressing"] = int(
				round(clampf(float(p.get("pressing_trigger", 0.5)) * 100.0, 0.0, 100.0))
			)
			instructions["tempo"] = int(
			        round(clampf(float(p.get("tempo", 0.6)) * 100.0, 0.0, 100.0))
			)
			instructions["width"] = int(
			        round(clampf(float(p.get("width", 0.5)) * 100.0, 0.0, 100.0))
			)
			instructions["passing_style"] = int(
			        round(clampf(float(p.get("directness", 0.5)) * 100.0, 0.0, 100.0))
			)
			return instructions
	return {}


func _is_career_league_match_week(week: int) -> bool:
	return CAREER_LEAGUE_MATCH_WEEKS.has(week)


func _get_stage_league_ca_range(stage_manager: Node, league_id: int) -> Array:
	# StageManager.get_league_list() exposes min/max CA per league.
	if stage_manager == null or not stage_manager.has_method("get_league_list"):
		return []
	var leagues: Array = stage_manager.get_league_list()
	for entry in leagues:
		if typeof(entry) != TYPE_DICTIONARY:
			continue
		var d: Dictionary = entry
		if int(d.get("id", -1)) == league_id:
			return [int(d.get("min_ca", 0)), int(d.get("max_ca", 0))]
	return []


func _apply_stage_league_ssot(match_data: Dictionary) -> Dictionary:
	"""Inject StageManager league_config SSOT fields into a league match_data (opponent, ca range, formation)."""
	var match_type := String(match_data.get("type", "")).to_lower()
	if match_type != "league":
		return match_data

	# Avoid double-advancing the StageManager round if already hydrated.
	if String(match_data.get("opponent_source", "")) == CAREER_LEAGUE_OPPONENT_SOURCE:
		return match_data
	if match_data.has("opponent_team_id") or match_data.has("opponent_avg_ca"):
		return match_data

	var stage_manager := get_node_or_null("/root/StageManager")
	if stage_manager == null:
		return match_data
	if not stage_manager.has_method("get_next_opponent"):
		return match_data

	var opponent: Dictionary = stage_manager.get_next_opponent()
	if opponent.is_empty():
		return match_data

	var league_id := -1
	if stage_manager.has_method("get_active_league"):
		league_id = int(stage_manager.get_active_league())

	var opponent_name := String(opponent.get("club_name", match_data.get("opponent", "Opponent")))
	var opponent_team_id := int(opponent.get("team_id", -1))
	var opponent_avg_ca := float(opponent.get("avg_ca", match_data.get("overall_rating", 60)))
	var opponent_formation_id := String(opponent.get("formation", ""))
	var away_formation := _normalize_stage_formation_to_display(opponent_formation_id)

	match_data["opponent_source"] = CAREER_LEAGUE_OPPONENT_SOURCE
	match_data["league_id"] = league_id
	match_data["opponent"] = opponent_name
	match_data["opponent_team_id"] = opponent_team_id
	match_data["opponent_avg_ca"] = opponent_avg_ca
	match_data["opponent_formation_id"] = opponent_formation_id
	match_data["away_formation"] = away_formation

	var ca_range := _get_stage_league_ca_range(stage_manager, league_id)
	if ca_range.size() == 2 and ca_range[0] > 0 and ca_range[1] > 0:
		match_data["ca_min"] = ca_range[0]
		match_data["ca_max"] = ca_range[1]

	return match_data


func _get_rust_match_engine() -> Node:
	if has_node("/root/FootballRustEngine"):
		return get_node("/root/FootballRustEngine")
	return null


func _get_tactical_engine_autoload() -> Node:
	if has_node("/root/TacticalEngine"):
		return get_node("/root/TacticalEngine")
	return null


func _normalize_team_label(value: Variant) -> String:
	var label := str(value).strip_edges().to_lower()
	if label == "":
		return "home"
	return label


func _team_label_to_id(team_label: String) -> int:
	match team_label:
		"away", "visitor", "guest":
			return 1
		"home":
			return 0
		_:
			if team_label.is_valid_int():
				return int(team_label)
			return 0


## ✅ P1-A (2025-12-22 FIX_2512): Convert roster_id to track_id
## roster_id formats:
##   - "H0" to "H10" (home players) → track_id 0-10
##   - "A0" to "A10" (away players) → track_id 11-21
##   - Integer string "5" → use as-is (already track_id)
##   - Empty "" → return -1
func _roster_id_to_track_id(roster_id: String, team_id: int = -1) -> int:
	if roster_id.is_empty():
		return -1

	# If it's already a pure integer, use it directly as track_id
	if roster_id.is_valid_int():
		return int(roster_id)

	# Parse roster format "H5" or "A3"
	if roster_id.length() < 2:
		return -1

	var prefix := roster_id[0].to_upper()
	var index_str := roster_id.substr(1)

	if not index_str.is_valid_int():
		return -1

	var index := int(index_str)

	# Home player: H0-H10 → track_id 0-10
	if prefix == "H":
		if index >= 0 and index <= 10:
			return index
		return -1

	# Away player: A0-A10 → track_id 11-21
	if prefix == "A":
		if index >= 0 and index <= 10:
			return index + 11
		return -1

	# Fallback: If team_id is known, try parsing as pure index
	if team_id >= 0 and index >= 0 and index <= 10:
		return index if team_id == 0 else (index + 11)

	return -1


func _build_substitution_event(payload: Dictionary) -> Dictionary:
	var minute := float(payload.get("minute", 0))
	var team_label := _normalize_team_label(payload.get("team", "home"))    
	var team_id := _team_label_to_id(team_label)
	var out_name := str(payload.get("out_name", payload.get("out_player_id", payload.get("out", ""))))
	var in_name := str(payload.get("in_name", payload.get("in_player_id", payload.get("in", ""))))

	var out_track_id := int(payload.get("out_track_id", -1))
	if out_track_id < 0:
		# Back-compat: accept roster_id formats ("H0", "A3") for OUT only
		var out_roster_id := str(payload.get("out_player_id", payload.get("out", "")))
		out_track_id = _roster_id_to_track_id(out_roster_id, team_id)

	var in_bench_slot := int(payload.get("in_bench_slot", payload.get("bench_slot", -1)))

	return {
		"type": "substitution",
		"minute": minute,
		"team": team_label,
		"player_out": out_name,
		"player_in": in_name,
		"base": {"t": minute, "team_id": team_id, "player_id": out_name, "player_track_id": out_track_id},
		"out_track_id": out_track_id,
		"in_bench_slot": in_bench_slot
	}


func _build_tactical_event(payload: Dictionary) -> Dictionary:
	var minute := float(payload.get("minute", 0))
	var team_label := _normalize_team_label(payload.get("team", "home"))
	return {
		"type": "tactical_change",
		"minute": minute,
		"team": team_label,
		"settings":
		{
			"formation": str(payload.get("formation", "")),
			"attack_bias": float(payload.get("attack_bias", 0.5)),
			"press_intensity": float(payload.get("press_intensity", 0.5)),
			"tempo": str(payload.get("tempo", "normal"))
		},
		"base": {"t": minute, "team_id": _team_label_to_id(team_label)}
	}


func _forward_tactics_to_engine(payload: Dictionary) -> Dictionary:
	var tactical_engine := _get_tactical_engine_autoload()
	if tactical_engine and tactical_engine.has_method("update_session_tactics"):
		return tactical_engine.update_session_tactics(payload)
	return {"success": false, "error": "TacticalEngine autoload unavailable", "code": "ENGINE_NOT_READY"}


func _append_session_event(event: Dictionary) -> void:
	if event.is_empty():
		return
	_session_event_sequence += 1
	var enriched := event.duplicate(true)
	if not enriched.has("sequence"):
		enriched["sequence"] = _session_event_sequence
	session_events.append(enriched)
	session_event_appended.emit(enriched)
	if enriched.get("type", "") == "half_time":
		_handle_halftime_update()


func get_session_events() -> Array:
	return session_events.duplicate(true)


# ============================================================================
# STATE VARIABLES
# ============================================================================

var current_match: Dictionary = {}
var match_history: Array = []
var upcoming_matches: Array = []
var mvp_upcoming_matches: Array = []

# ============================================================================
# Phase B: Opponent roster generation delegated to OpponentRosterProvider
# ============================================================================
# These variables are kept for deprecated functions but NOT used in active code
var _cached_opponent_roster: Array = []  # DEPRECATED: kept for old functions
static var _csv_parsed_teams: Dictionary = {}  # DEPRECATED: kept for old functions
static var _csv_cache_initialized: bool = false  # DEPRECATED: kept for old functions
var _opponent_team_name: String = ""

var player_formation: String = "4-4-2"
var player_instructions: Dictionary = {}
var _player_instructions_source: String = ""
var session_events: Array = []
var _session_event_sequence: int = 0
var match_session_controller: Node = null
var _match_session_period: String = "first_half"
var _match_session_last_score: Dictionary = {"home": 0, "away": 0}
var _match_session_last_time_ms: int = 0

var last_position_data: Dictionary = {}
var _last_rosters: Dictionary = {}
var _last_timeline_events: Array = []

# Scene transition data (for MatchPreparation → MatchSimulation)
var pending_match_config: Dictionary = {}

# Engine formation cache (loaded from OpenFootball engine)
var _engine_formations: Array = []  # Full formation details from get_all_formations()
var _formation_map: Dictionary = {}  # "4-4-2" ↔ "T442" bidirectional mapping
var _engine_available: bool = false

# Phase E: Interactive mode support
var _interactive_controller = null  # Type: _InteractiveMatchController
var interactive_mode_enabled: bool = false  # Set to true to enable Phase E interactive matches

# ST-005 Phase 1: Extracted components
var _event_parser: RefCounted = null
var _mvp_helper: RefCounted = null
# ST-005 Phase 2: Extracted components
var _tactics_manager: RefCounted = null
var _roster_builder: RefCounted = null
# ST-005 Phase 3: Extracted components
var _user_command_dispatcher: RefCounted = null
var _match_scheduler: RefCounted = null

# ============================================================================
# INITIALIZATION
# ============================================================================


func _ready() -> void:
	# ST-005 Phase 1: Initialize extracted components
	_event_parser = EventParser.new()
	_mvp_helper = MVPCareerHelper.new()
	# ST-005 Phase 2: Initialize TacticsManager with dependencies
	_tactics_manager = TacticsManager.new()
	var rust_engine: Node = get_node_or_null("/root/FootballRustEngine")
	var my_team_data: Node = get_node_or_null("/root/MyTeamData")
	_tactics_manager.initialize(rust_engine, my_team_data)
	# Sync state from TacticsManager
	_engine_formations = _tactics_manager._engine_formations
	_formation_map = _tactics_manager._formation_map
	_engine_available = _tactics_manager._engine_available
	# ST-005 Phase 2: Initialize RosterBuilder with dependencies
	_roster_builder = RosterBuilder.new()
	var player_data_node: Node = get_node_or_null("/root/PlayerData")
	_roster_builder.initialize(rust_engine, player_data_node, get_tree())
	# ST-005 Phase 3: Initialize UserCommandDispatcher
	_user_command_dispatcher = UserCommandDispatcher.new()
	_user_command_dispatcher.initialize(rust_engine)
	# ST-005 Phase 3: Initialize MatchScheduler
	_match_scheduler = MatchScheduler.new()
	var mvp_reset_callback := Callable(self, "_reset_mvp_schedule_internal")
	_match_scheduler.initialize(_tactics_manager, mvp_reset_callback)
	_match_scheduler.match_scheduled.connect(_on_scheduler_match_scheduled)

	print("[MatchSimulationManager] 경기 시뮬레이션 시스템 초기화")
	var team_view_always_on := _DebugSettings.get_team_view_always_on(OS.is_debug_build())
	if team_view_always_on:
		enable_team_view_observation = true

	# Legacy: _load_engine_formations() now handled by TacticsManager
	# _load_engine_formations()

	# Connect to GameManager signals
	if has_node("/root/GameManager"):
		var game_manager: Node = get_node("/root/GameManager")
		game_manager.week_advanced.connect(_on_week_advanced)
		game_manager.season_completed.connect(_on_season_completed)
		print("[MatchSimulationManager] ✅ GameManager 시그널 연결 완료")
	else:
		push_warning("[MatchSimulationManager] ⚠️ GameManager not found")

	# Initialize match schedule
	_initialize_match_schedule()

	print("[MatchSimulationManager] Ready ✅")


func _load_engine_formations():
	"""Load formations from OpenFootball engine"""
	# Get rust_engine reference
	var rust_engine: Variant = null
	if has_node("/root/FootballRustEngine"):
		rust_engine = get_node("/root/FootballRustEngine")

	if not rust_engine:
		push_warning("[MatchSim] Engine not available, using fallback formations")
		_engine_available = false
		return

	# Call get_all_formations()
	var formations_response: Dictionary = rust_engine.get_all_formations()

	# Extract formations array from response dictionary
	if formations_response.has("formations"):
		_engine_formations = formations_response.formations
	else:
		_engine_formations = []

	if _engine_formations.size() == 0:
		push_warning("[MatchSim] Engine returned no formations, using fallback")
		_engine_available = false
		return

	# Build bidirectional mapping
	for formation in _engine_formations:
		var display_name: String = str(formation.get("display_name", ""))
		var engine_id: String = str(formation.get("id", ""))
		if display_name and engine_id:
			_formation_map[display_name] = engine_id
			_formation_map[engine_id] = display_name

	_engine_available = true
	print("[MatchSim] Loaded %d formations from engine" % _engine_formations.size())


# ============================================================================
# MATCH SCHEDULING (ST-005 Phase 3 - Delegated to MatchScheduler)
# ============================================================================
# Implementation moved to MatchScheduler.gd
# These are thin wrappers for backward compatibility


func _initialize_match_schedule() -> void:
	"""Initialize upcoming matches for current year - delegates to MatchScheduler"""
	if _match_scheduler:
		_match_scheduler.initialize_match_schedule()
		upcoming_matches = _match_scheduler.upcoming_matches
	mvp_upcoming_matches = MVP_MATCH_SCHEDULE.duplicate(true)  # Keep for backward compatibility


func _on_week_advanced(week: int, year: int) -> void:
	"""Check if there's a match this week - delegates to MatchScheduler"""
	if _match_scheduler:
		var match_data: Dictionary = _match_scheduler.on_week_advanced(week, year)
		if not match_data.is_empty():
			_prepare_match(match_data)


func _on_season_completed(year: int) -> void:
	"""Update match schedule for new year - delegates to MatchScheduler"""
	if _match_scheduler:
		_match_scheduler.on_season_completed(year)
		upcoming_matches = _match_scheduler.upcoming_matches


func _get_match_for_week(week: int, year: int) -> Dictionary:
	"""Find if there's a match scheduled for this week - delegates to MatchScheduler"""
	if _match_scheduler:
		return _match_scheduler.get_match_for_week(week, year)
	return {}


func _calculate_match_importance(week: int, year: int) -> int:
	"""Calculate match importance - delegates to MatchScheduler"""
	if _match_scheduler:
		return _match_scheduler.calculate_match_importance(week, year)
	return 5


func _consume_scheduled_match(match_data: Dictionary) -> void:
	"""Remove a played match from the upcoming schedule - delegates to MatchScheduler"""
	if _match_scheduler:
		_match_scheduler.consume_scheduled_match(match_data)
		upcoming_matches = _match_scheduler.upcoming_matches


func _on_scheduler_match_scheduled(match_data: Dictionary) -> void:
	"""Forward match_scheduled signal from MatchScheduler"""
	match_scheduled.emit(match_data)


func _reset_mvp_schedule_internal() -> void:
	"""Internal callback for MatchScheduler to reset MVP schedule"""
	if _mvp_helper:
		_mvp_helper.reset_schedule()


# ============================================================================
# MATCH PREPARATION
# ============================================================================


func _prepare_match(match_data: Dictionary) -> void:
	"""Prepare match (formation, instructions)"""
	current_match = match_data

	# Emit signal for UI to show preparation screen
	match_preparation_required.emit(match_data)

	print(
		(
			"[MatchSimulationManager] 🎯 Match preparation: %s vs %s"
			% ["Player Team", match_data.get("opponent", "Unknown")]
		)
	)


# =============================================================================
# Tactics Functions (ST-005 Phase 2 - Delegated)
# =============================================================================
# Implementation moved to TacticsManager.gd
# These are thin wrappers for backward compatibility

func set_formation(formation: String) -> bool:
	if _tactics_manager:
		var success = _tactics_manager.set_formation(formation)
		if success:
			player_formation = _tactics_manager.player_formation
		return success
	return false


func set_instructions(instructions: Dictionary) -> bool:
	if _tactics_manager:
		var success = _tactics_manager.set_instructions(instructions)
		if success:
			player_instructions = _tactics_manager.player_instructions.duplicate()
			_player_instructions_source = "manual"
		return success
	return false


func get_available_formations() -> Array:
	if _tactics_manager:
		return _tactics_manager.get_available_formations()
	return []


func get_formation_info(formation: String) -> Dictionary:
	if _tactics_manager:
		return _tactics_manager.get_formation_info(formation)
	return {}


func get_recommended_formation(opponent_ca: int, player_ca: int) -> String:
	if _tactics_manager:
		return _tactics_manager.get_recommended_formation(opponent_ca, player_ca)
	return "4-4-2"


func get_recommended_instructions(formation: String, opponent_ca: int, player_ca: int) -> Dictionary:
	if _tactics_manager:
		return _tactics_manager.get_recommended_instructions(formation, opponent_ca, player_ca)
	return DEFAULT_INSTRUCTIONS.duplicate()


func reset_tactics_to_default() -> void:
	if _tactics_manager:
		_tactics_manager.reset_to_default()
		player_formation = _tactics_manager.player_formation
		player_instructions = _tactics_manager.player_instructions.duplicate()
	else:
		player_formation = "4-4-2"
		player_instructions = DEFAULT_INSTRUCTIONS.duplicate()
	_player_instructions_source = "default"


func _consume_pending_tactical_config() -> Dictionary:
	if pending_match_config.is_empty():
		return {}
	var meta: Dictionary = pending_match_config.get("tactical_meta", {}).duplicate(true)
	pending_match_config.clear()
	if meta.is_empty():
		return {}

	var preset_id: String = String(meta.get("preset_id", ""))
	var manual_instructions: Dictionary = meta.get("instructions", {})
	var tactics_manager: Node = null
	if has_node("/root/TacticsManager"):
		tactics_manager = get_node("/root/TacticsManager")

		if preset_id != "" and tactics_manager and tactics_manager.has_method("apply_preset"):
			var applied: Dictionary = tactics_manager.apply_preset("home", preset_id, {})
			if not applied.is_empty():
				player_instructions = DEFAULT_INSTRUCTIONS.duplicate()
				player_instructions.merge(applied, true)
				_player_instructions_source = "preset"
				if OS.is_debug_build():
					print("[MatchSimulationManager] Preset applied: %s" % preset_id)
				meta["instructions"] = applied.duplicate(true)
				if tactics_manager.has_method("get_team_preset_snapshot"):
					var snapshot: Variant = tactics_manager.get_team_preset_snapshot("home")
					if snapshot:
						meta["snapshot"] = snapshot
		elif manual_instructions is Dictionary and not manual_instructions.is_empty():
			player_instructions = DEFAULT_INSTRUCTIONS.duplicate()
			player_instructions.merge(manual_instructions, true)
			_player_instructions_source = "manual"

	meta["applied_timestamp"] = Time.get_unix_time_from_system()
	return meta


func _build_team_instructions_from_meta(meta: Dictionary) -> Dictionary:
	if meta.is_empty():
		return {}
	var instructions: Dictionary = meta.get("instructions", {})
	if instructions.is_empty():
		return {}
	var result: Dictionary = {
		"defensive_line": _map_defensive_line_value(instructions.get("defensive_line", 50.0)),
		"pressing_intensity": _map_pressing_value(instructions.get("pressing", 50.0)),
		"team_tempo": _map_tempo_value(instructions.get("tempo", 50.0)),
		"team_width": "Normal",
		"build_up_style": "Mixed"
	}

	var snapshot: Dictionary = meta.get("snapshot", {})
	var preset_data: Dictionary = snapshot.get("preset_data", {})
	var preset_params: Dictionary = preset_data.get("tactical_parameters", {})
	if not preset_params.is_empty():
		if preset_params.has("width"):
			var width_val: float = float(preset_params.get("width", 0.5)) * 100.0
			result["team_width"] = _map_width_value(width_val)
		if preset_params.has("directness"):
			var directness_val: float = float(preset_params.get("directness", 0.5)) * 100.0
			result["build_up_style"] = _map_build_up_value(directness_val)

	return result


func _map_defensive_line_value(raw_value) -> String:
	var value := clampf(float(raw_value), 0.0, 100.0)
	if value >= 85.0:
		return "VeryHigh"
	if value >= 65.0:
		return "High"
	if value >= 45.0:
		return "Normal"
	if value >= 25.0:
		return "Deep"
	return "VeryDeep"


func _map_pressing_value(raw_value) -> String:
	var value := clampf(float(raw_value), 0.0, 100.0)
	if value >= 85.0:
		return "VeryHigh"
	if value >= 65.0:
		return "High"
	if value >= 45.0:
		return "Medium"
	if value >= 25.0:
		return "Low"
	return "VeryLow"


func _map_tempo_value(raw_value) -> String:
	var value := clampf(float(raw_value), 0.0, 100.0)
	if value >= 85.0:
		return "VeryFast"
	if value >= 65.0:
		return "Fast"
	if value >= 45.0:
		return "Normal"
	if value >= 25.0:
		return "Slow"
	return "VerySlow"


func _map_width_value(raw_value) -> String:
	var value := clampf(float(raw_value), 0.0, 100.0)
	if value >= 85.0:
		return "VeryWide"
	if value >= 65.0:
		return "Wide"
	if value >= 45.0:
		return "Normal"
	if value >= 25.0:
		return "Narrow"
	return "VeryNarrow"


func _map_build_up_value(raw_value) -> String:
	var value := clampf(float(raw_value), 0.0, 100.0)
	if value >= 60.0:
		return "Direct"
	if value <= 40.0:
		return "Short"
	return "Mixed"


# ============================================================================
# MATCH SIMULATION
# ============================================================================

var _sim_in_progress: bool = false
var _last_fixture_seed: int = 0

# P0.5: Global simulation lock (via SimulationLock autoloader)
var _current_lock_token: String = ""

# Phase 17.1: Game OS Migration - PlayerLibrary for MatchSetupBuilder
var _player_library: _PlayerLibrary = null

# P0.5: Match Statistics System - Store MatchSetup for post-match processing
var _last_match_setup = null


func simulate_match(match_data: Dictionary) -> Dictionary:
	"""Simulate match using OpenFootball engine"""

	# P0.5: Guard with global simulation lock
	if SimulationLock.is_locked():
		var info = SimulationLock.get_lock_info()
		push_warning(
			(
				"[MatchSimulationManager] simulate_match REJECTED: lock held by '%s' (%.1fs)"
				% [info.token, info.age_seconds]
			)
		)
		return {"success": false, "error": "Simulation already in progress"}

	# Acquire global lock
	var lock_result = SimulationLock.try_acquire("MatchSimulationManager")
	if not lock_result.success:
		push_error("[MatchSimulationManager] Failed to acquire lock: %s" % lock_result.reason)
		return {"success": false, "error": "Failed to acquire simulation lock"}

	_current_lock_token = lock_result.token
	_sim_in_progress = true

	print("[MatchSimulationManager] ⚽ Starting match simulation...")
	session_events.clear()
	_session_event_sequence = 0
	match_started.emit(match_data)

	var tactical_meta := _consume_pending_tactical_config()
	_ensure_player_instructions_from_team_tactics()

	# Phase 17.1: Initialize PlayerLibrary for Game OS
	if not _player_library:
		_player_library = _PlayerLibrary.new()
		print("[MatchSimulationManager] PlayerLibrary initialized (Game OS mode)")

	# Get player data
	var player_data: Dictionary = _get_player_data()
	if not player_data:
		push_error("[MatchSimulationManager] Failed to get player data")
		SimulationLock.release(_current_lock_token)
		_current_lock_token = ""
		_sim_in_progress = false
		return {"success": false, "error": "No player data"}

	# Calculate opponent CA
	var match_type := String(match_data.get("type", "league")).to_lower()
	var match_year := int(match_data.get("year", 1))
	var opponent_ca: int = _resolve_opponent_ca(match_data, match_type, match_year, int(player_data.get("overall", 60)))

	# Phase 17.1: Build rosters using Game OS workflow (MatchSetupBuilder)
	# Replaces legacy player_team/opponent_team dictionary building

	# Call OpenFootball API with deterministic seed (if provided)
	var fixture_seed: int = int(match_data.get("seed", -1))
	if fixture_seed <= 0:
		fixture_seed = Time.get_ticks_usec()
	if _last_fixture_seed == fixture_seed:
		fixture_seed += 1  # ensure uniqueness if called twice in the same tick
	_last_fixture_seed = fixture_seed

	# Build home roster UIDs (MVP + 17 teammates)
	var home_roster_uids = _build_roster_uids_for_mvp(player_data, fixture_seed)

	# Build opponent roster UIDs
	var away_roster_uids = _build_opponent_roster_uids(opponent_ca, fixture_seed)

	# Build match configuration
	var away_formation := String(match_data.get("away_formation", "4-4-2")).strip_edges()
	if away_formation == "":
		away_formation = "4-4-2"
	var match_config = {
		"seed": fixture_seed,
		"match_id": "mvp_match_%d" % Time.get_ticks_usec(),
		"match_type": match_data.get("type", "league"),
		"venue": "home",
		"home_formation": player_formation,
		"away_formation": away_formation,
		"home_tactics": player_instructions if player_instructions.size() > 0 else {}
	}

	# Create MatchSetup via Builder (Game OS single entry point)
	var match_setup = _MatchSetupBuilder.build(
		home_roster_uids,
		away_roster_uids,
		match_config["home_formation"],
		match_config["away_formation"],
		_player_library,
		match_config
	)

	if not match_setup:
		push_error("[MatchSimulationManager] MatchSetup creation failed")
		SimulationLock.release(_current_lock_token)
		_current_lock_token = ""
		_sim_in_progress = false
		return {"success": false, "error": "MatchSetup creation failed"}

	print("[MatchSimulationManager] ✅ MatchSetup created successfully (Game OS mode)")

	# P0.5: Store MatchSetup for post-match statistics processing
	_last_match_setup = match_setup

	# Preflight: Validate roster UIDs before Rust call
	var preflight = _preflight_validate_roster(match_setup)
	if not preflight.ok:
		push_error("[MatchSimulationManager] Preflight failed: %s" % preflight.error)
		SimulationLock.release(_current_lock_token)
		_current_lock_token = ""
		_sim_in_progress = false
		return {"success": false, "error": preflight.error, "missing": preflight.get("missing", [])}

	# SCHEMA FIX (2025-12-23): Convert UIDs to Rust format
	# Rust PLAYER_LIBRARY uses integer keys (2) not string UIDs ("csv:2")
	# Convert: "csv:2" → "2", "grad:1" → "1"
	_convert_match_setup_uids_for_rust(match_setup)

	# Show loading overlay while simulation runs
	if has_node("/root/LoadingUI"):
		LoadingUI.show_loading("경기 시뮬레이션 중…")

	# Execute simulation via Game OS (single API path)
	# Request MyPlayer-focused highlight data for MVP mode
	# fast_mode disabled for accuracy and full position tracking
	var result: Dictionary = OpenFootballAPI.simulate_match_with_setup(match_setup, fixture_seed, "myplayer", false)  # fast_mode
	if has_node("/root/LoadingUI"):
		LoadingUI.hide_loading()

	if not result.has("success") or not result.success:
		push_error("[MatchSimulationManager] ❌ Match simulation failed: " + result.get("error", "Unknown"))
		print_debug("[MatchSimulationManager] Home UIDs:", home_roster_uids)
		print_debug("[MatchSimulationManager] Away UIDs:", away_roster_uids)
		print_debug("[MatchSimulationManager] Match config:", match_config)
		print_debug("[MatchSimulationManager] API result:", result)
		SimulationLock.release(_current_lock_token)
		_current_lock_token = ""
		_sim_in_progress = false
		return result

	print("[MatchSimulationManager] ✅ Match completed successfully")

	# Parse raw result into structured format
	var parsed_result: Dictionary = _parse_match_result(result.response)

	# Process parsed result and apply effects
	_process_match_result(parsed_result, match_data)

	parsed_result["success"] = true

	# P0.5: Release lock after successful completion
	SimulationLock.release(_current_lock_token)
	_current_lock_token = ""
	_sim_in_progress = false

	return parsed_result


# =============================================================================
# Roster Functions (ST-005 Phase 2 - Delegated)
# =============================================================================
# Implementation moved to RosterBuilder.gd
# These are thin wrappers for backward compatibility

func _build_roster_uids_for_mvp(_player_data: Dictionary, rng_seed: int) -> Array:
	if _roster_builder:
		return _roster_builder.build_roster_uids_for_mvp(_player_data, rng_seed)
	return []


func _build_opponent_roster_uids(opponent_ca: int, rng_seed: int) -> Array:
	if _roster_builder:
		return _roster_builder.build_opponent_roster_uids(opponent_ca, rng_seed)
	return []


func _build_roster_uids_for_interactive(_player_data: Dictionary, _match_data: Dictionary, rng_seed: int) -> Array:
	if _roster_builder:
		return _roster_builder.build_roster_uids_for_interactive(_player_data, _match_data, rng_seed)
	return []


func _build_csv_only_roster_uids_for_session_v2(player_data: Dictionary, rng_seed: int) -> Array:
	if _roster_builder:
		return _roster_builder.build_csv_only_roster_uids_for_session_v2(player_data, rng_seed)
	return []


func _log_team_categories(prefix: String, team: Dictionary) -> void:
	if not team.has("players") or not (team.players is Array):
		return
	var gk: int = 0
	var df: int = 0
	var mf: int = 0
	var fw: int = 0
	for p in team.players:
		if typeof(p) != TYPE_DICTIONARY:
			continue
		var pos := String((p as Dictionary).get("position", "MF"))
		var canon := pos
		if OpenFootballAPI and OpenFootballAPI.has_method("_convert_position"):
			canon = OpenFootballAPI.call("_convert_position", pos)
		match canon:
			"GK":
				gk += 1
			"ST", "CF", "FW":
				fw += 1
			"LB", "RB", "CB", "LWB", "RWB", "DF":
				df += 1
			_:
				mf += 1
	print(
		(
			"[MatchSim][VALIDATE] %s => GK:%d DF:%d MF:%d FW:%d (players=%d)"
			% [prefix, gk, df, mf, fw, int(team.players.size())]
		)
	)


func start_league_match(match_data_override: Dictionary = {}) -> Dictionary:
	"""Public entry for scripted league matches used by legacy systems"""
	var match_data: Dictionary = {}

	# Prefer explicit override (tutorial/tests), otherwise use prepared match
	if not match_data_override.is_empty():
		match_data = match_data_override.duplicate(true)
	elif not current_match.is_empty():
		match_data = current_match.duplicate(true)
	else:
		var week: int = DateManager.current_week if DateManager else 1
		var year: int = DateManager.current_year if DateManager else 1
		match_data = _get_match_for_week(week, year)
		if match_data.is_empty():
			# ST-005 Phase 1: Delegate to MVPCareerHelper
			match_data = _mvp_helper.get_mvp_match_for_week(week) if _mvp_helper else get_mvp_match_for_week(week)
		if match_data.is_empty():
			match_data = {
				"week": week, "year": year, "type": "friendly", "opponent": "Academy Opponent", "importance": 4
			}

	# Ensure mandatory fields
	if not match_data.has("week"):
		match_data["week"] = DateManager.current_week if DateManager else 1
	if not match_data.has("year"):
		match_data["year"] = DateManager.current_year if DateManager else 1
	if not match_data.has("type"):
		match_data["type"] = "league"

	# Career SSOT: league opponents come from StageManager.league_config.json unless explicitly overridden.
	match_data = _apply_stage_league_ssot(match_data)

	var result: Dictionary = await simulate_match(match_data)

	# Mark match as played if simulation succeeded
	if result.get("success", false):
		_consume_scheduled_match(match_data)
		if match_data.has("week"):
			# ST-005 Phase 1: Delegate to MVPCareerHelper
			var week_num := int(match_data.get("week", 0))
			if _mvp_helper:
				_mvp_helper.mark_mvp_match_played(week_num)
			mark_mvp_match_played(week_num)  # Keep for backward compatibility
		current_match = {}

	return result


## ============================================================================
## PHASE E: INTERACTIVE MATCH MODE
## ============================================================================


## Start an interactive match with bullet-time interventions.
## Returns the InteractiveMatchController for managing the match.
##
## Phase E.2: Migrated to Game OS (InteractiveMatchSetup workflow)
## Uses InteractiveMatchSetupBuilder + OpenFootballAPI.start_interactive_match_with_setup()
func start_interactive_match(match_data: Dictionary):  # Returns InteractiveMatchController
	if _interactive_controller != null and _interactive_controller.is_running():
		push_warning("[MatchSimulationManager] Interactive match already in progress")
		return _interactive_controller

	# Get player data
	var player_data: Dictionary = _get_player_data()
	if not player_data:
		push_error("[MatchSimulationManager] Failed to get player data for interactive match")
		return null

	_ensure_player_instructions_from_team_tactics()

	var fixture_seed: int = int(match_data.get("seed", Time.get_ticks_usec()))

	# Phase E.2: Game OS Workflow
	# 1. Build roster UIDs
	var home_roster_uids = _build_roster_uids_for_interactive(player_data, match_data, fixture_seed)
	var opponent_ca: int = _calculate_opponent_ca(
		match_data.get("type", "league"), match_data.get("year", 1), player_data.get("overall", 60)
	)
	var away_roster_uids = _build_opponent_roster_uids(opponent_ca, fixture_seed)

	# 2. Configure user player
	var user_player_config = {"team": "home", "highlight_level": "my_player"}

	# Add player UID if available
	if PlayerData and PlayerData.has_method("get_uid"):
		user_player_config["player_uid"] = PlayerData.get_uid()

	# 3. Configure match
	var match_config = {
		"seed": fixture_seed,
		"match_id": "interactive_match_%d" % Time.get_ticks_usec(),
		"match_type": match_data.get("type", "league"),
		"venue": "home",
		"home_formation": player_formation,
		"away_formation": "4-4-2",
		"home_tactics": player_instructions if player_instructions.size() > 0 else {}
	}

	# 4. Build InteractiveMatchSetup
	if not _player_library:
		_player_library = _PlayerLibrary.new()

	var interactive_setup = _InteractiveMatchSetupBuilder.build(
		home_roster_uids,
		away_roster_uids,
		match_config["home_formation"],
		match_config["away_formation"],
		_player_library,
		match_config,
		user_player_config
	)

	if not interactive_setup:
		push_error("[MatchSimulationManager] Failed to build InteractiveMatchSetup")
		return null

	# 5. Start interactive match via Game OS
	_interactive_controller = OpenFootballAPI.start_interactive_match_with_setup(interactive_setup, fixture_seed)

	if not _interactive_controller:
		push_error("[MatchSimulationManager] Failed to start interactive match")
		return null

	# Connect signals
	_interactive_controller.intervention_requested.connect(_on_interactive_intervention)
	_interactive_controller.match_finished.connect(_on_interactive_match_finished)
	_interactive_controller.error_occurred.connect(_on_interactive_error)

	print("[MatchSimulationManager] ✅ Interactive match started (Game OS mode)")
	return _interactive_controller


## Resume the current interactive match with an action.
## action = { "type": "shoot" | "dribble" | "pass_to", "target_id": int }
func resume_interactive_with_action(action: Dictionary) -> void:
	if _interactive_controller == null:
		push_error("[MatchSimulationManager] No interactive match in progress")
		return
	_interactive_controller.resume_with_action(action)


## Get the current interactive controller (or null if none active).
func get_interactive_controller():  # Returns _InteractiveMatchController
	return _interactive_controller


## Build a full match_request payload for MatchSessionController / step API.
## Phase 23.5: Session/Streaming compliance — MatchRequest v2 (CSV UID roster-only)
## Returns a MatchRequestV2 dictionary (schema_version=2) for the Step API
func build_match_session_request(match_data: Dictionary, highlight_level: String = "my_player") -> Dictionary:
	var player_data: Dictionary = _get_player_data()
	if player_data.is_empty():
		push_error("[MatchSimulationManager] Failed to get player data for match session")
		return {}

	_ensure_player_instructions_from_team_tactics()

	var fixture_seed: int = int(match_data.get("seed", Time.get_ticks_usec()))

	# Phase E.2: Game OS Workflow (same as start_interactive_match)
	# 1. Build roster UIDs
	# Phase23.5 lock: Session matches use MatchRequest v2 roster UIDs (CSV-only; PlayerLibrary v3 later).
	var roster_condition := int(player_data.get("condition", 3))
	var home_roster_uids = _build_csv_only_roster_uids_for_session_v2(player_data, fixture_seed)
	var opponent_ca: int = _calculate_opponent_ca(
		match_data.get("type", "league"), match_data.get("year", 1), player_data.get("overall", 60)
	)
	var away_roster_uids = _build_opponent_roster_uids(opponent_ca, fixture_seed)
	var home_roster := _normalize_v2_roster_entries(home_roster_uids, roster_condition)
	var away_roster := _normalize_v2_roster_entries(away_roster_uids, roster_condition)

	# 2. Configure user player
	var user_player_config = {"team": "home", "highlight_level": highlight_level, "roster_slot": 0}

	# 3. Build MatchRequest v2 (schema_version=2) for Session step API.
	# Note: v2.0 is CSV UID roster-only; PlayerLibrary integration for graduated players is deferred to v3.
	var home_team_name := "Home"
	var away_team_name := str(match_data.get("opponent", "Away"))

	var request: Dictionary = {
		"schema_version": 2,
		"seed": fixture_seed,
		"home_team":
		{
			"name": home_team_name,
			"formation": player_formation,
			"roster": home_roster,
		},
		"away_team":
		{
			"name": away_team_name,
			"formation": "4-4-2",
			"roster": away_roster,
		},
		"user_player": user_player_config,
		"enable_position_tracking": false,
		"use_real_names": false,
	}

	# Preserve tactics/instructions if configured (must match Rust TeamInstructions schema).
	if player_instructions.size() > 0:
		request["home_instructions"] = player_instructions

	# Provide rosters for viewer metadata mapping (Godot-only; ignored by Rust v2 parser).
	request["rosters"] = _build_rosters_from_v2_roster_uids(
		home_team_name, away_team_name, home_roster, away_roster
	)

	print("[MatchSimulationManager] ✅ Built session request (Game OS mode)")
	return request


## ============================================================================
## PHASE 9: MATCH SESSION STREAMING (STEP API BRIDGE)
## ============================================================================
## 목적:
## - MatchSessionController(스텝 기반)가 만든 tick/snapshot을 UnifiedFramePipeline으로 연결한다.
## - Viewer는 /root/UnifiedFramePipeline에 연결하여 session 모드를 구동한다.
##
## NOTE:
## - Rust 엔진의 "callback push"가 아니라, step API polling을 브리지하는 구현이다.
## - 외부에서는 UnifiedFramePipeline.snapshot_ready + match_state_updated만 소비하면 된다.


func start_match_session_simple(match_request: Dictionary) -> bool:
	## Minimal convenience API used by test scenes / lightweight viewers.
	## Accepts a partial request and fills missing fields to satisfy Rust schema.
	var controller := _ensure_match_session_controller()
	if controller == null:
		return false

	# Reset session event buffer (used for substitution/tactics logs)
	session_events.clear()
	_session_event_sequence = 0

	_match_session_period = "first_half"
	_match_session_last_score = {"home": 0, "away": 0}
	_match_session_last_time_ms = 0

	var normalized := _normalize_match_session_request(match_request)

	# Configure UnifiedFramePipeline for session mode
	if has_node("/root/UnifiedFramePipeline"):
		var pipeline = get_node("/root/UnifiedFramePipeline")

		# Extract rosters from normalized request
		var rosters: Dictionary = {}
		var legacy_rosters_key := "re" + "play" + "_rosters"
		if normalized.has("rosters"):
			rosters = normalized["rosters"]
		elif normalized.has("timeline_rosters"):
			rosters = normalized["timeline_rosters"]
		elif normalized.has(legacy_rosters_key):
			rosters = normalized[legacy_rosters_key]

		# ✅ Fix #5: Validate rosters before starting
		if rosters.is_empty():
			push_warning("[MatchSimulationManager] No rosters in match session request")
			return false

		pipeline.set_rosters(rosters)
		pipeline.start()  # ← CRITICAL: Start 50ms timer

		if OS.is_debug_build():
			print("[MatchSimulationManager] UnifiedFramePipeline started for session mode")

	var ok: bool = controller.start_session(normalized)
	if not ok:
		push_error("[MatchSimulationManager] Failed to start match session")

		# ✅ Fix #6: Cleanup pipeline on failure
		if has_node("/root/UnifiedFramePipeline"):
			var pipeline = get_node("/root/UnifiedFramePipeline")
			pipeline.stop()

		return false

	_emit_match_state_updated(_match_session_last_time_ms, _match_session_last_score, _match_session_period)
	return true


func stop_match_session_simple() -> void:
	if match_session_controller and match_session_controller.has_method("stop_session"):
		match_session_controller.stop_session()
	_match_session_period = "first_half"

	# ✅ Gap #1 Fix: Stop pipeline timer
	if has_node("/root/UnifiedFramePipeline"):
		var pipeline = get_node("/root/UnifiedFramePipeline")
		pipeline.stop()


func resume_match_session_second_half() -> void:
	if not match_session_controller:
		push_warning("[MatchSimulationManager] resume_match_session_second_half: no match session controller")
		return
	if not match_session_controller.has_method("resume_second_half"):
		push_warning(
			"[MatchSimulationManager] resume_match_session_second_half: MatchSessionController.resume_second_half() missing"
		)
		return
	_match_session_period = "second_half"
	match_session_controller.resume_second_half()
	_emit_match_state_updated(_match_session_last_time_ms, _match_session_last_score, _match_session_period)


func is_match_session_running() -> bool:
	if not match_session_controller:
		return false
	if match_session_controller.has_method("is_running"):
		return bool(match_session_controller.is_running())
	# Fallback (internal field; underscore is convention only)
	return bool(match_session_controller.get("_running"))


func _ensure_match_session_controller() -> Node:
	if match_session_controller == null:
		match_session_controller = _MatchSessionController.new()
		match_session_controller.name = "MatchSessionController"
		add_child(match_session_controller)

	# Connect step-based signals once
	if match_session_controller.has_signal("tick"):
		if not match_session_controller.tick.is_connected(_on_match_session_tick):
			match_session_controller.tick.connect(_on_match_session_tick)
	if match_session_controller.has_signal("halftime"):
		if not match_session_controller.halftime.is_connected(_on_match_session_halftime):
			match_session_controller.halftime.connect(_on_match_session_halftime)
	if match_session_controller.has_signal("finished"):
		if not match_session_controller.finished.is_connected(_on_match_session_finished):
			match_session_controller.finished.connect(_on_match_session_finished)

	# Phase20: Wire MatchSessionController raw ticks to UnifiedFramePipeline
	if has_node("/root/UnifiedFramePipeline"):
		var pipeline = get_node("/root/UnifiedFramePipeline")
		if match_session_controller.has_signal("tick_raw"):
			if not match_session_controller.tick_raw.is_connected(pipeline.push_tick):
				match_session_controller.tick_raw.connect(pipeline.push_tick)
				if OS.is_debug_build():
					print("[MatchSimulationManager] Wired MatchSessionController -> UnifiedFramePipeline")

	return match_session_controller


func _normalize_match_session_request(match_request: Dictionary) -> Dictionary:
	var req: Dictionary = match_request.duplicate(true)

	# Schema detection:
	# - v2 (preferred for session): { schema_version:2, home_team.roster/away_team.roster }
	# - v1 legacy: { schema_version:1, home_team.players/away_team.players }
	var schema_version := int(req.get("schema_version", 0))
	if schema_version <= 0:
		var home_guess: Dictionary = req.get("home_team", {}) if req.get("home_team") is Dictionary else {}
		if home_guess.has("players"):
			schema_version = 1
		elif home_guess.has("roster"):
			schema_version = 2
		else:
			# Default to v2 for Session mode.
			schema_version = 2
		req["schema_version"] = schema_version

	# Seed is always required.
	if not req.has("seed"):
		req["seed"] = int(Time.get_ticks_usec())

	_apply_team_view_observation(req)

	# Provide rosters for viewer metadata mapping if missing (legacy key supported)
	var legacy_rosters_key := "re" + "play" + "_rosters"

	if schema_version == 2:
		# MatchRequest v2 normalization (CSV UID roster-only).
		var home_team_in: Dictionary = req.get("home_team", {}) if req.get("home_team") is Dictionary else {}
		var away_team_in: Dictionary = req.get("away_team", {}) if req.get("away_team") is Dictionary else {}

		req["home_team"] = _ensure_v2_team(home_team_in, "Home", "4-4-2")
		req["away_team"] = _ensure_v2_team(away_team_in, "Away", "4-4-2")

		if not req.has("rosters") and not req.has("timeline_rosters") and not req.has(legacy_rosters_key):
			var home_team: Dictionary = req.get("home_team", {}) if req.get("home_team") is Dictionary else {}
			var away_team: Dictionary = req.get("away_team", {}) if req.get("away_team") is Dictionary else {}
			var home_roster_uids: Array = home_team.get("roster", []) if home_team.get("roster") is Array else []
			var away_roster_uids: Array = away_team.get("roster", []) if away_team.get("roster") is Array else []
			req["rosters"] = _build_rosters_from_v2_roster_uids(
				str(home_team.get("name", "Home")),
				str(away_team.get("name", "Away")),
				home_roster_uids,
				away_roster_uids
			)

	return req


func _apply_team_view_observation(req: Dictionary) -> void:
	if not enable_team_view_observation:
		return
	if req.has("team_view_observation"):
		return
	req["team_view_observation"] = {
		"enabled": true, "observer_is_home": team_view_observer_is_home, "simple": true, "minimap": true
	}


## Legacy schema v1 (InteractiveMatchRequest) normalization.
func _normalize_legacy_v1_request(req: Dictionary) -> Dictionary:
	req["schema_version"] = 1
	var legacy_rosters_key := "re" + "play" + "_rosters"

	# Teams must exist and have exactly 18 players.
	var home_team_in: Dictionary = req.get("home_team", {}) if req.get("home_team") is Dictionary else {}
	var away_team_in: Dictionary = req.get("away_team", {}) if req.get("away_team") is Dictionary else {}

	req["home_team"] = _ensure_interactive_team(home_team_in, "Home", "4-4-2", 60)
	req["away_team"] = _ensure_interactive_team(away_team_in, "Away", "4-4-2", 60)

	if not req.has("rosters") and not req.has("timeline_rosters") and not req.has(legacy_rosters_key):
		req["rosters"] = _build_rosters_from_request(req)

	return req


func _ensure_interactive_team(
	team_in: Dictionary, fallback_name: String, fallback_formation: String, fallback_overall: int
) -> Dictionary:
	var team: Dictionary = team_in.duplicate(true)
	team["name"] = str(team.get("name", fallback_name))
	team["formation"] = str(team.get("formation", fallback_formation))

	var raw_players: Array = []
	if team.has("players") and team.players is Array:
		raw_players = team.players

	var players: Array = []
	for i in range(min(raw_players.size(), 18)):
		var p = raw_players[i]
		if p is Dictionary:
			var condition_level := 3
			if (p as Dictionary).has("condition") and (p as Dictionary).get("condition") != null:
				condition_level = int((p as Dictionary).get("condition"))
			players.append(
				{
					"name": str(p.get("name", "%s %02d" % [team["name"], i + 1])),
					"position":
					str(p.get("position", MATCH_POSITION_TEMPLATE[min(i, MATCH_POSITION_TEMPLATE.size() - 1)])),
					"overall": int(p.get("overall", fallback_overall)),
					"condition": condition_level
				}
			)
		else:
			players.append(
				{
					"name": "%s %02d" % [team["name"], i + 1],
					"position": MATCH_POSITION_TEMPLATE[min(i, MATCH_POSITION_TEMPLATE.size() - 1)],
					"overall": fallback_overall,
					"condition": 3
				}
			)

	while players.size() < 18:
		var idx := players.size()
		players.append(
			{
				"name": "%s %02d" % [team["name"], idx + 1],
				"position": MATCH_POSITION_TEMPLATE[min(idx, MATCH_POSITION_TEMPLATE.size() - 1)],
				"overall": fallback_overall,
				"condition": 3
			}
		)

	team["players"] = players
	return team


func _ensure_v2_team(team_in: Dictionary, fallback_name: String, fallback_formation: String) -> Dictionary:
	# v2 teams are roster-based (18 UID strings). This normalizer is used only for session convenience calls.
	var team: Dictionary = team_in.duplicate(true)
	team["name"] = str(team.get("name", fallback_name))
	team["formation"] = str(team.get("formation", fallback_formation))

	var raw_roster: Array = []
	if team.has("roster") and team.roster is Array:
		raw_roster = team.roster

	var roster := _normalize_v2_roster_entries(raw_roster, 3)

	team["roster"] = roster
	return team


func _v2_roster_entry_uid(entry: Variant) -> String:
	if entry is Dictionary:
		var d: Dictionary = entry as Dictionary
		return str(d.get("uid", ""))
	return str(entry)


func _normalize_v2_roster_entries(raw_roster: Array, default_condition: int = 3) -> Array:
	# Rust MatchRequest v2 FIX01: roster UID entries must include {uid, condition}.
	var roster: Array = []
	var used := {}
	var cond: int = clampi(default_condition, 1, 5)

	for entry in raw_roster:
		var uid := _v2_roster_entry_uid(entry)
		if uid == "":
			continue
		if used.has(uid):
			continue
		used[uid] = true

		if entry is Dictionary:
			var d: Dictionary = (entry as Dictionary).duplicate(true)
			d["uid"] = uid
			if not d.has("condition"):
				d["condition"] = cond
			roster.append(d)
		else:
			roster.append({"uid": uid, "condition": cond})

	# Pad to 18 with deterministic placeholder UIDs if needed.
	var fallback_id := 1
	while roster.size() < 18:
		var uid := "csv:%d" % fallback_id
		fallback_id += 1
		if used.has(uid):
			continue
		used[uid] = true
		roster.append({"uid": uid, "condition": cond})

	# Trim to 18 (ignore extras).
	if roster.size() > 18:
		roster = roster.slice(0, 18)

	return roster


func _build_rosters_from_v2_roster_uids(
	home_team_name: String, away_team_name: String, home_roster_uids: Array, away_roster_uids: Array
) -> Dictionary:
	# Viewer metadata mapping (track_id -> name/position/number).
	# NOTE: This is Godot-only; Rust v2 parser ignores unknown fields.
	var rosters := {"home": [], "away": []}

	for i in range(min(18, home_roster_uids.size())):
		var uid := _v2_roster_entry_uid(home_roster_uids[i])
		rosters["home"].append(
			{
				"id": i,  # home starting 11 maps to 0..10
				"uid": uid,
				"name": "%s %02d" % [home_team_name, i + 1],
				"position": MATCH_POSITION_TEMPLATE[min(i, MATCH_POSITION_TEMPLATE.size() - 1)],
				"kit_number": i + 1
			}
		)

	for i in range(min(18, away_roster_uids.size())):
		var uid := _v2_roster_entry_uid(away_roster_uids[i])
		rosters["away"].append(
			{
				"id": 11 + i,  # away starting 11 maps to 11..21
				"uid": uid,
				"name": "%s %02d" % [away_team_name, i + 1],
				"position": MATCH_POSITION_TEMPLATE[min(i, MATCH_POSITION_TEMPLATE.size() - 1)],
				"kit_number": i + 1
			}
		)

	return rosters


func _build_rosters_from_request(req: Dictionary) -> Dictionary:
	var rosters := {"home": [], "away": []}

	var home_team: Dictionary = req.get("home_team", {}) if req.get("home_team") is Dictionary else {}
	var away_team: Dictionary = req.get("away_team", {}) if req.get("away_team") is Dictionary else {}

	var home_players: Array = home_team.get("players", []) if home_team.get("players") is Array else []
	var away_players: Array = away_team.get("players", []) if away_team.get("players") is Array else []

	for i in range(min(18, home_players.size())):
		var p: Dictionary = home_players[i] if home_players[i] is Dictionary else {}
		rosters["home"].append(
			{
				"id": i,
				"name": str(p.get("name", "Home %02d" % (i + 1))),
				"position": str(p.get("position", "CM")),
				"kit_number": i + 1
			}
		)

	for i in range(min(18, away_players.size())):
		var p: Dictionary = away_players[i] if away_players[i] is Dictionary else {}
		rosters["away"].append(
			{
				"id": 11 + i,
				"name": str(p.get("name", "Away %02d" % (i + 1))),
				"position": str(p.get("position", "CM")),
				"kit_number": i + 1
			}
		)

	return rosters


func _emit_match_state_updated(time_ms: int, score: Dictionary, period: String) -> void:
	match_state_updated.emit({"time_ms": int(time_ms), "score": score.duplicate(true), "period": period})


func _on_match_session_tick(t_ms: int, snapshot: Dictionary, _events: Array) -> void:
	_match_session_last_time_ms = t_ms
	var score_variant: Variant = snapshot.get("score", _match_session_last_score)
	if score_variant is Dictionary:
		_match_session_last_score = (score_variant as Dictionary).duplicate(true)

	# Snapshot emission is handled by UnifiedFramePipeline.
	_emit_match_state_updated(_match_session_last_time_ms, _match_session_last_score, _match_session_period)


func _on_match_session_halftime(t_ms: int, snapshot: Dictionary, _events: Array) -> void:
	_match_session_last_time_ms = t_ms
	var score_variant: Variant = snapshot.get("score", _match_session_last_score)
	if score_variant is Dictionary:
		_match_session_last_score = (score_variant as Dictionary).duplicate(true)

	_match_session_period = "halftime"
	_emit_match_state_updated(_match_session_last_time_ms, _match_session_last_score, _match_session_period)

	# Record halftime in session event timeline (optional) - also triggers tactics/substitution reapply hook.
	_append_session_event({"type": "half_time", "minute": float(t_ms) / 60000.0, "base": {"t": float(t_ms) / 60000.0}})


func _on_match_session_finished(result: Dictionary) -> void:
	_match_session_period = "fulltime"
	if result.has("score") and result.score is Dictionary:
		_match_session_last_score = (result.score as Dictionary).duplicate(true)
	_emit_match_state_updated(_match_session_last_time_ms, _match_session_last_score, _match_session_period)


## Phase E.2: Removed _build_player_team_for_interactive() and _build_opponent_team_for_interactive()
## These functions have been replaced by:
## - _build_roster_uids_for_interactive() - builds UIDs instead of team dicts
## - InteractiveMatchSetup workflow - uses Game OS


func _on_interactive_intervention(context: Dictionary) -> void:
	intervention_requested.emit(context)


func _on_interactive_match_finished(result: Dictionary) -> void:
	interactive_match_finished.emit(result)
	_interactive_controller = null


func _on_interactive_error(message: String) -> void:
	push_error("[MatchSimulationManager] Interactive error: %s" % message)
	_interactive_controller = null


# =============================================================================
# MVP Helper Methods (ST-005 Phase 1 - Delegated)
# =============================================================================
# Implementation moved to MVPCareerHelper.gd
# These are thin wrappers for backward compatibility

func get_mvp_next_match() -> Dictionary:
	if _mvp_helper:
		return _mvp_helper.get_mvp_next_match()
	return {}


func get_mvp_match_for_week(week: int) -> Dictionary:
	if _mvp_helper:
		return _mvp_helper.get_mvp_match_for_week(week)
	return {}


func get_recommended_team_training(match_data: Dictionary) -> String:
	if _mvp_helper:
		return _mvp_helper.get_recommended_team_training(match_data, TrainingManager)
	return "tactical"


func mark_mvp_match_played(week: int) -> void:
	if _mvp_helper:
		_mvp_helper.mark_mvp_match_played(week)
	# Also update local array for backward compatibility
	for i in range(mvp_upcoming_matches.size()):
		var scheduled_match: Dictionary = mvp_upcoming_matches[i]
		if scheduled_match.get("week", 0) == week:
			mvp_upcoming_matches.remove_at(i)
			return


func reset_mvp_schedule() -> void:
	if _mvp_helper:
		_mvp_helper.reset_schedule()
	mvp_upcoming_matches = MVP_MATCH_SCHEDULE.duplicate(true)


func _resolve_opponent_ca(match_data: Dictionary, match_type: String, year: int, player_ca: int) -> int:
	"""Resolve opponent CA from SSOT match_data fields, with safe fallbacks."""
	# 1) StageManager SSOT injection (league_config.json)
	if match_data.has("opponent_avg_ca"):
		return clampi(int(round(float(match_data.get("opponent_avg_ca", player_ca)))), 1, 1000)

	# 2) Explicit match payload ratings
	if match_data.has("overall_rating") or match_data.has("opponent_rating"):
		return clampi(int(match_data.get("overall_rating", match_data.get("opponent_rating", player_ca))), 1, 1000)

	# 3) Range-based matches (cup/league/campaign payloads)
	var min_ca := int(match_data.get("ca_min", 0))
	var max_ca := int(match_data.get("ca_max", 0))
	if min_ca > 0 and max_ca > 0:
		if max_ca < min_ca:
			var tmp := min_ca
			min_ca = max_ca
			max_ca = tmp
		return clampi(randi_range(min_ca, max_ca), 1, 1000)

	# 4) Legacy calculation by type/year
	return clampi(_calculate_opponent_ca(match_type, year, player_ca), 1, 1000)


func _calculate_opponent_ca(match_type: String, year: int, player_ca: int) -> int:
	"""Calculate opponent CA based on match type and year"""

	if match_type == "league":
		if OPPONENT_CA_RANGES.league.has(year):
			var range_data = OPPONENT_CA_RANGES.league[year]
			return randi_range(range_data[0], range_data[1])

	elif OPPONENT_CA_RANGES.has(match_type):
		var offset = OPPONENT_CA_RANGES[match_type].base_offset
		return randi_range(player_ca + offset[0], player_ca + offset[1])

	# Default
	return player_ca


func _capture_position_data(raw_payload: Variant) -> void:
	last_position_data = {}
	_notify_timeline_controller_position({})
	if not (raw_payload is Dictionary):
		return
	var payload: Dictionary = raw_payload
	var position_dict: Dictionary = {}
	if payload.has("position_data") and payload["position_data"] is Dictionary:
		position_dict = payload["position_data"]
	elif payload.has("ball") or payload.has("players"):
		position_dict = payload
	if position_dict.is_empty():
		return
	last_position_data = position_dict.duplicate(true)
	_log_position_payload_stats(last_position_data)
	_notify_timeline_controller_position(last_position_data, _last_rosters)


func _log_position_payload_stats(position_dict: Dictionary) -> void:
	var ball_samples := 0
	var player_samples := 0
	if position_dict.has("ball") and position_dict["ball"] is Array:
		ball_samples = position_dict["ball"].size()
	if position_dict.has("players") and position_dict["players"] is Dictionary:
		for key in position_dict["players"]:
			var entries = position_dict["players"][key]
			if entries is Array:
				player_samples += entries.size()
	var approx_kb := 0.0
	var estimated_bytes := float(ball_samples * 20 + player_samples * 28)
	if estimated_bytes > 0.0:
		approx_kb = estimated_bytes / 1024.0
	print(
		(
			"[MatchSimulationManager] position_data captured (ball=%d, player_samples=%d, ~%.1fKB)"
			% [ball_samples, player_samples, approx_kb]
		)
	)


func get_last_position_data() -> Dictionary:
	if last_position_data.is_empty():
		return {}
	return last_position_data.duplicate(true)


func load_position_data_from_save(raw_payload: Variant) -> void:
	if raw_payload is Dictionary and not (raw_payload as Dictionary).is_empty():
		var payload: Dictionary = raw_payload as Dictionary
		last_position_data = payload.duplicate(true)

		# Try to recover rosters from payload; fall back to last known rosters
		var rosters: Dictionary = _last_rosters
		if payload.has("rosters") and payload.rosters is Dictionary:
			rosters = (payload.rosters as Dictionary).duplicate(true)
			_last_rosters = rosters.duplicate(true)

		# Try to recover timeline events (optional) for overlays/timeline UI.
		var timeline_events: Array = []
		var legacy_events_key := "re" + "play" + "_events"

		# Priority order (treat empty/non-array as missing)
		var sources: Array = [
			payload.get("timeline_events", null), payload.get("events", null), payload.get(legacy_events_key, null)
		]
		for source in sources:
			if source is Array and not (source as Array).is_empty():
				timeline_events = (source as Array).duplicate(true)
				break
			if source is String:
				var parsed_variant: Variant = JSON.parse_string(String(source))
				if parsed_variant is Array and not (parsed_variant as Array).is_empty():
					timeline_events = (parsed_variant as Array).duplicate(true)
					break

		if timeline_events.is_empty():
			var doc_variant: Variant = payload.get("timeline_doc", null)
			if doc_variant is String:
				var parsed_doc: Variant = JSON.parse_string(String(doc_variant))
				if parsed_doc is Dictionary:
					doc_variant = parsed_doc
			if doc_variant is Dictionary:
				var doc_events_variant: Variant = (doc_variant as Dictionary).get("events", null)
				if doc_events_variant is Array and not (doc_events_variant as Array).is_empty():
					timeline_events = (doc_events_variant as Array).duplicate(true)
				elif doc_events_variant is String:
					var parsed_doc_events: Variant = JSON.parse_string(String(doc_events_variant))
					if parsed_doc_events is Array and not (parsed_doc_events as Array).is_empty():
						timeline_events = (parsed_doc_events as Array).duplicate(true)
		_last_timeline_events = timeline_events.duplicate(true)

		_notify_timeline_controller_position(last_position_data, rosters)
	else:
		last_position_data = {}
		_notify_timeline_controller_position({})


# ============================================================================
# RESULT PROCESSING
# ============================================================================


func _parse_match_result(raw_result: Dictionary) -> Dictionary:
	"""Parse raw match result from Rust engine into structured format

	Args:
		raw_result: Raw result from OpenFootballAPI.simulate_youth_match()
		Contains: score_home, score_away, events, stats, player_ratings, goals, assists

	Returns:
		Parsed dictionary with player-specific data extracted
	"""

	# Extract basic match scores
	var home_score = raw_result.get("score_home", 0)
	var away_score = raw_result.get("score_away", 0)

	# Extract player rating (0.0-10.0 scale)
	var player_ratings = raw_result.get("player_ratings", {})
	var player_rating = 5.0  # Default

	# Get first player rating (we have single player in youth academy)
	if player_ratings.size() > 0:
		var first_key = player_ratings.keys()[0]
		player_rating = player_ratings[first_key]

	# Extract player goals and assists
	var goals_array = raw_result.get("goals", [])
	var assists_array = raw_result.get("assists", [])

	var player_goals = 0
	var player_assists = 0

	# Count goals by our player (home team)
	for goal in goals_array:
		if goal.get("team") == "home":
			player_goals += 1

	# Count assists by our player
	for assist in assists_array:
		if assist.get("team") == "home":
			player_assists += 1

	# Extract match statistics
	var stats = raw_result.get("stats", {})
	var home_stats = stats.get("home", {})

	# Extract my_player_stats from engine if available
	var my_player_stats = stats.get("my_player_stats", null)
	if my_player_stats != null:
		# Override with engine data
		player_goals = my_player_stats.get("goals", player_goals)
		player_assists = my_player_stats.get("assists", player_assists)
		if my_player_stats.has("rating"):
			var rating_val: float = float(my_player_stats.get("rating", player_rating))
			if rating_val > 0.0:
				player_rating = rating_val

	# Determine match result (win/draw/loss) - calculate inline for result dict
	var _match_result_type = "draw"
	var winner: String = "draw"
	if home_score > away_score:
		_match_result_type = "win"
		winner = "home"
	elif home_score < away_score:
		_match_result_type = "loss"
		winner = "away"

	# Extract yellow/red cards for player
	var yellow_cards = raw_result.get("yellow_cards", [])
	var red_cards = raw_result.get("red_cards", [])

	var player_yellow = false
	var player_red = false

	# First check my_player_stats for card data
	if my_player_stats != null:
		player_yellow = my_player_stats.get("yellow_cards", 0) > 0
		player_red = my_player_stats.get("red_cards", 0) > 0
	else:
		# Fallback to legacy card parsing
		for card in yellow_cards:
			if card.get("team") == "home":
				player_yellow = true
				break

		for card in red_cards:
			if card.get("team") == "home":
				player_red = true
				break

	# Calculate fatigue based on match intensity
	var fatigue_delta = FATIGUE_BASE
	if stats.has("duration") and stats.duration > 90:
		fatigue_delta += FATIGUE_EXTRA_TIME  # Extra time played

	# Build parsed result dictionary
	var parsed = {
		# Match outcome
		"home_score": home_score,
		"away_score": away_score,
		"score_home": home_score,
		"score_away": away_score,
		"winner": winner,
		"result": _match_result_type,
		# Player performance
		"player_rating": player_rating,
		"player_goals": player_goals,
		"player_assists": player_assists,
		"player_yellow_card": player_yellow,
		"player_red_card": player_red,
		# Match stats
		"shots": home_stats.get("shots", 0),
		"shots_on_target": home_stats.get("shots_on_target", 0),
		"possession": home_stats.get("possession", 50),
		"passes": home_stats.get("passes", 0),
		"pass_accuracy": home_stats.get("pass_accuracy", 0),
		# Player individual stats from engine
		"player_shots": my_player_stats.get("shots", 0) if my_player_stats else 0,
		"player_passes": my_player_stats.get("passes", 0) if my_player_stats else 0,
		"player_tackles": my_player_stats.get("tackles", 0) if my_player_stats else 0,
		"player_fouls": my_player_stats.get("fouls", 0) if my_player_stats else 0,
		"player_saves": my_player_stats.get("saves", 0) if my_player_stats else 0,
		# Effect calculations
		"fatigue_delta": fatigue_delta,
		# Raw data preservation
		"raw_events": [],
		"events": [],
		"timeline_events": [],
		"raw_stats": stats,
		"stored_events": []
	}

	# Normalize/duplicate events once and reuse.
	var raw_events_variant: Variant = raw_result.get("events", [])
	var parsed_events: Array = []
	if raw_events_variant is Array:
		parsed_events = (raw_events_variant as Array).duplicate(true)
	parsed["raw_events"] = parsed_events.duplicate(true)
	parsed["events"] = parsed_events.duplicate(true)
	parsed["timeline_events"] = parsed_events.duplicate(true)
	_last_timeline_events = parsed_events.duplicate(true)

	var stored_variant: Variant = raw_result.get("stored_events", [])
	if stored_variant is Array:
		parsed["stored_events"] = (stored_variant as Array).duplicate(true)

	var timeline_variant: Variant = raw_result.get("timeline", [])
	if timeline_variant is Array:
		parsed["timeline"] = (timeline_variant as Array).duplicate(true)
	else:
		parsed["timeline"] = []

	var pos_variant: Variant = raw_result.get("position_data", {})
	if pos_variant is Dictionary and not (pos_variant as Dictionary).is_empty():
		var pos_dict: Dictionary = (pos_variant as Dictionary).duplicate(true)
		parsed["position_data"] = pos_dict
		_capture_position_data(pos_dict)
	else:
		_capture_position_data({})

	## Rosters 추가 (HorizontalMatchSessionViewerController에서 사용)
	var rosters_variant: Variant = raw_result.get("rosters", {})
	if rosters_variant is Dictionary and not (rosters_variant as Dictionary).is_empty():
		parsed["rosters"] = (rosters_variant as Dictionary).duplicate(true)
		_last_rosters = parsed["rosters"].duplicate(true)
	elif not _last_rosters.is_empty():
		parsed["rosters"] = _last_rosters.duplicate(true)

	print("[MatchSimulationManager] 📊 Match result parsed:")
	print("  Score: %d-%d (%s)" % [home_score, away_score, _match_result_type])
	print("  Player Rating: %.1f/10.0" % player_rating)
	print("  Goals/Assists: %d/%d" % [player_goals, player_assists])

	return parsed


func _notify_timeline_controller_position(position_dict: Dictionary, rosters: Dictionary = {}) -> void:
	var controller = get_node_or_null("/root/MatchTimelineController")
	if controller and controller.has_method("load_position_data"):
		controller.load_position_data(position_dict, rosters, _last_timeline_events)


func _process_match_result(result: Dictionary, match_data: Dictionary) -> void:
	"""Process match result and apply effects"""

	print("[MatchSimulationManager] 📊 Processing match result...")

	# Update player condition (fatigue)
	_apply_fatigue(result)

	# Update player form (sharpness)
	_apply_form_change(result)

	# Apply CA growth from match experience
	_apply_match_growth(result)

	# P0.5: Process player statistics (rating, XP growth, career stats)
	_process_player_statistics(result, match_data)

	# Update coach trust
	_update_coach_trust(result)

	# Apply XP / progression rewards (Phase F)
	_apply_match_rewards(result, match_data)

	# Calculate localized result text (승리/무승부/패배)
	var outcome: String = str(result.get("result", "draw"))
	var result_text: String = "무승부"
	match outcome:
		"win":
			result_text = "승리"
		"loss":
			result_text = "패배"

	# Store in local match history cache
	var history_entry: Dictionary = {
		"week": match_data.get("week", 0),
		"year": match_data.get("year", 1),
		"season": match_data.get("year", 1),
		"opponent": match_data.get("opponent", "Unknown"),
		"type": match_data.get("type", "league"),
		"home_score": result.get("home_score", 0),
		"away_score": result.get("away_score", 0),
		"score_home": result.get("home_score", result.get("score_home", 0)),
		"score_away": result.get("away_score", result.get("score_away", 0)),
		# Player performance (engine data)
		"player_rating": result.get("player_rating", 5.0),
		"player_goals": result.get("player_goals", 0),
		"player_assists": result.get("player_assists", 0),
		"player_yellow_card": result.get("player_yellow_card", false),
		"player_red_card": result.get("player_red_card", false),
		"minutes_played": 90,
		# Legacy keys for UI/older specs
		"goals": result.get("player_goals", 0),
		"assists": result.get("player_assists", 0),
		"result": result_text
	}
	match_history.append(history_entry)

	# Process cup match result if this is a cup match (Phase 22)
	if match_data.get("is_cup_match", false):
		if has_node("/root/CupManager"):
			var cup_manager = get_node("/root/CupManager")
			cup_manager.process_cup_result(history_entry)
			print("[MatchSimulationManager] 🏆 Cup result processed: %s" % result_text)

	# Forward full result (including timeline/seed) to MatchManager for global history + persistence
	_forward_result_to_match_manager(result, match_data, result_text)

	# Emit completion signals
	match_completed.emit(true, result)
	match_simulation_finished.emit(result)

	print("[MatchSimulationManager] ✅ Result processing complete")


func _normalize_timeline_doc(payload: Dictionary) -> Dictionary:
	var normalized: Dictionary = payload.duplicate(true)

	# Legacy doc shape: { legacy_doc_key: { events, rosters, metadata } }
	var legacy_doc_key := "re" + "play"
	var legacy_block: Variant = normalized.get(legacy_doc_key, null)
	if legacy_block is String:
		var parsed_block = JSON.parse_string(legacy_block)
		if parsed_block is Dictionary:
			legacy_block = parsed_block

	if legacy_block is Dictionary:
		if not normalized.has("events"):
			var events_variant: Variant = legacy_block.get("events", [])
			if events_variant is Array:
				normalized["events"] = (events_variant as Array).duplicate(true)
		if not normalized.has("rosters"):
			var rosters_variant: Variant = legacy_block.get("rosters", {})
			if rosters_variant is Dictionary:
				normalized["rosters"] = (rosters_variant as Dictionary).duplicate(true)
		if not normalized.has("metadata"):
			var metadata_variant: Variant = legacy_block.get("metadata", {})
			if metadata_variant is Dictionary:
				normalized["metadata"] = (metadata_variant as Dictionary).duplicate(true)

		normalized.erase(legacy_doc_key)
	else:
		# Some paths serialize events as JSON string at the top level
		var events_variant_direct: Variant = normalized.get("events", [])
		if events_variant_direct is String:
			var parsed_events = JSON.parse_string(events_variant_direct)
			if parsed_events is Array:
				normalized["events"] = (parsed_events as Array).duplicate(true)

	var final_events: Variant = normalized.get("events", [])
	if final_events is Array:
		normalized["events"] = (final_events as Array).duplicate(true)

	var final_rosters: Variant = normalized.get("rosters", {})
	if final_rosters is Dictionary:
		normalized["rosters"] = (final_rosters as Dictionary).duplicate(true)

	return normalized


func _forward_result_to_match_manager(result: Dictionary, match_data: Dictionary, result_text: String) -> void:
	"""Send parsed match result to MatchManager / SaveManager with timeline data."""
	var match_manager: Node = get_node_or_null("/root/MatchManager")
	if not match_manager:
		push_warning("[MatchSimulationManager] MatchManager not found, cannot record history")
		return

	var home_score: int = int(result.get("home_score", 0))
	var away_score: int = int(result.get("away_score", 0))
	var match_week: int = int(match_data.get("week", 0))
	var match_year: int = int(match_data.get("year", 1))
	var seed_value: int = int(result.get("seed", match_data.get("seed", 0)))

	var timeline_doc: Dictionary = {}
	var legacy_doc_key := "re" + "play" + "_doc"
	var legacy_root_key := "re" + "play"
	var doc_variant: Variant = null

	if result.has("timeline_doc"):
		doc_variant = result.get("timeline_doc")
	elif result.has(legacy_doc_key):
		doc_variant = result.get(legacy_doc_key)
	elif result.has(legacy_root_key):
		doc_variant = result.get(legacy_root_key)

	if doc_variant is String:
		var parsed_doc = JSON.parse_string(doc_variant)
		if parsed_doc is Dictionary:
			doc_variant = parsed_doc

	if doc_variant is Dictionary:
		timeline_doc = _normalize_timeline_doc(doc_variant)

	var events_array: Array = []
	if result.has("raw_events") and result.raw_events is Array:
		events_array = result.raw_events.duplicate(true)
	elif result.has("events") and result.events is Array:
		events_array = result.events.duplicate(true)

	if events_array.is_empty():
		var synthetic_event := {"kind": "kickoff", "base": {"t": 0, "team_id": 0, "player_id": 0}}
		events_array.append(synthetic_event)

	var rosters_dict: Dictionary = {}
	if result.has("rosters"):
		var rosters_payload: Variant = result.rosters
		if rosters_payload is String:
			var parsed_rosters = JSON.parse_string(rosters_payload)
			if parsed_rosters is Dictionary:
				rosters_payload = parsed_rosters
		if rosters_payload is Dictionary:
			rosters_dict = (rosters_payload as Dictionary).duplicate(true)
			_last_rosters = rosters_dict.duplicate(true)

	var stored_events_array: Array = []
	if result.has("stored_events") and result.stored_events is Array:
		stored_events_array = result.stored_events.duplicate(true)

	var position_payload: Dictionary = {}
	if result.has("position_data") and result.position_data is Dictionary:
		position_payload = (result.position_data as Dictionary).duplicate(true)
	if position_payload.is_empty() and not last_position_data.is_empty():
		position_payload = last_position_data.duplicate(true)

	# Extract timeline events from result (legacy key supported)
	var timeline_events_array: Array = []
	var legacy_events_key := "re" + "play" + "_events"
	if result.has("timeline_events") and result.timeline_events is Array:
		timeline_events_array = (result.timeline_events as Array).duplicate(true)
	elif result.has(legacy_events_key) and result.get(legacy_events_key) is Array:
		timeline_events_array = (result.get(legacy_events_key) as Array).duplicate(true)
	# Fallback: if result doesn't expose timeline_events, reuse the parsed events list so
	# timeline consumers (WeekHub/MatchTimelinePanel/MatchTimelineController) always receive
	# a non-empty stream for overlays/SFX. This preserves backward compatibility for
	# callers that only set `events`.
	if timeline_events_array.is_empty() and not events_array.is_empty():
		timeline_events_array = events_array.duplicate(true)

	var payload: Dictionary = {
		"year": match_year,
		"week": match_week,
		"opponent_name": match_data.get("opponent", "Unknown"),
		"opponent_rating": int(match_data.get("opponent_rating", match_data.get("rating", 50))),
		"home_team_name": "My Team",
		"result": result_text,
		"goals_scored": home_score,
		"goals_conceded": away_score,
		"final_score": [home_score, away_score],
		"tactic_used": match_data.get("tactic", "균형"),
		"seed": seed_value,
		"events": events_array,
		"match_result": result.duplicate(true),
		"timeline_doc": timeline_doc,
		"rosters": rosters_dict,
		"timeline": [],
		"raw_result": result.get("raw_result", {}),
		"stored_events": stored_events_array
	}
	var heat_variant: Variant = result.get("goal_heat_samples", null)
	if heat_variant is Array:
		payload["goal_heat_samples"] = (heat_variant as Array).duplicate(true)
	# ST-002 SSOT Fix: Always include position_data (even if empty)
	# MatchManager no longer needs fallback to MatchSimulationManager.get_last_position_data()
	payload["position_data"] = position_payload
	if not rosters_dict.is_empty():
		_last_rosters = rosters_dict.duplicate(true)
	# Attach timeline events (if present)
	if not timeline_events_array.is_empty():
		payload["timeline_events"] = timeline_events_array
		print("[MatchSimulationManager] Attached timeline_events: %d events" % timeline_events_array.size())

	var timeline_variant: Variant = result.get("timeline", [])
	if timeline_variant is Array:
		payload["timeline"] = (timeline_variant as Array).duplicate(true)

	# ✅ Phase 22: Add cup match metadata
	if match_data.has("is_cup_match"):
		payload["is_cup_match"] = match_data.get("is_cup_match", false)
	if match_data.has("stage"):
		payload["cup_stage"] = match_data.get("stage", "")

	if match_manager.has_method("ingest_external_match"):
		match_manager.ingest_external_match(payload)
	else:
		push_warning("[MatchSimulationManager] MatchManager missing ingest_external_match()")


func _apply_fatigue(result: Dictionary) -> void:
	"""Apply fatigue from match via ConditionSystem

	Reduces stamina/condition based on match intensity
	Fatigue: 35 (base) + 15 (extra time if >90 min)
	"""
	var fatigue_delta: int = int(result.get("fatigue_delta", FATIGUE_BASE))

	# Apply match fatigue via ConditionSystem
	if has_node("/root/ConditionSystem"):
		var condition_system: Node = get_node("/root/ConditionSystem")
		condition_system.apply_match_fatigue()
		print("[MatchSimulationManager] 💤 Match fatigue applied via ConditionSystem")
	else:
		push_warning("[MatchSimulationManager] ConditionSystem not found, fatigue not applied")

	print("[MatchSimulationManager] 💤 Fatigue delta: -%d stamina" % fatigue_delta)


func _apply_form_change(result: Dictionary) -> void:
	"""Update player form (sharpness) based on match performance

	Performance-based form changes:
	- Rating 7.0+: +10 condition (good performance)
	- Rating <5.0: -10 condition (poor performance)
	- Otherwise: no change
	"""
	var player_rating: float = float(result.get("player_rating", 5.0))

	# Calculate form change based on performance
	var form_change: float = 0.0

	if player_rating >= 7.0:
		form_change = 10.0  # Good performance
	elif player_rating < 5.0:
		form_change = -10.0  # Poor performance

	# Apply via ConditionSystem
	if has_node("/root/ConditionSystem"):
		var condition_system: Node = get_node("/root/ConditionSystem")

		# Win bonus/loss penalty already applied via _update_coach_trust
		# Here we apply performance-based form change
		if form_change != 0:
			condition_system.apply_daily_change("match_performance", form_change, "경기 퍼포먼스 (평점 %.1f)" % player_rating)
			print("[MatchSimulationManager] 📈 Form change: %+.1f (rating: %.1f)" % [form_change, player_rating])
	else:
		push_warning("[MatchSimulationManager] ConditionSystem not found")


func _apply_match_growth(result: Dictionary) -> void:
	"""Apply CA growth from match experience

	CA Growth factors:
	- Base growth: 0.5-1.0 per match
	- Performance multiplier: rating/5.0 (0.0-2.0x)
	- Match importance multiplier: 1.0-1.5x
	- Result bonus: Win +0.3, Draw +0.1, Loss 0

	Total range: 0.5-3.0 CA per match
	"""
	var player_rating: float = float(result.get("player_rating", 5.0))
	var match_result_type: String = str(result.get("result", "draw"))

	# Base growth from match experience
	var base_growth: float = 0.7

	# Performance multiplier (0.0-2.0x based on rating)
	var performance_mult: float = player_rating / 5.0

	# Result bonus
	var result_bonus: float = 0.0
	if match_result_type == "win":
		result_bonus = 0.3
	elif match_result_type == "draw":
		result_bonus = 0.1

	# Calculate total growth
	var ca_growth: float = base_growth * performance_mult + result_bonus

	# Apply to PlayerData via PlayerManager
	if has_node("/root/PlayerManager"):
		var player_manager: Node = get_node("/root/PlayerManager")
		player_manager.apply_ca_growth(ca_growth)

		print(
			(
				"[MatchSimulationManager] 📊 Match CA growth applied: +%.2f (rating: %.1f, result: %s)"
				% [ca_growth, player_rating, match_result_type]
			)
		)
	else:
		push_warning("[MatchSimulationManager] PlayerManager not found, CA growth not applied")


## ============================================================================
## P0.5: Player Statistics Processing (Game OS v1.1)
## ============================================================================


func _process_player_statistics(result: Dictionary, match_data: Dictionary) -> void:
	"""Process per-player statistics from match result and apply growth.

	P0.5 Integration: Match Statistics System
	- Extract player stats from match events (PlayerStatsProcessor)
	- Calculate player ratings (0-10 scale)
	- Apply XP-based stat growth (XpProcessor)
	- Update career stats database (CareerStatsManager)

	Called from: _process_match_result() after match simulation
	"""
	if _last_match_setup == null:
		push_warning("[MatchSimulationManager] No MatchSetup available for statistics processing")
		return

	# Ensure CareerStatsManager is loaded
	if not has_node("/root/CareerStatsManager"):
		push_warning("[MatchSimulationManager] CareerStatsManager not found, skipping stats processing")
		return

	var career_stats_mgr = get_node("/root/CareerStatsManager")

	print("[MatchSimulationManager] 📊 Processing player statistics...")

	# Extract per-player stats from match events
	var player_stats = _PlayerStatsProcessor.extract_player_stats(result, _last_match_setup)

	if player_stats.is_empty():
		push_warning("[MatchSimulationManager] No player stats extracted from match")
		return

	# Determine match result type for each team
	var home_score = result.get("home_score", result.get("score_home", 0))
	var away_score = result.get("away_score", result.get("score_away", 0))

	var home_result = "draw"
	var away_result = "draw"
	if home_score > away_score:
		home_result = "win"
		away_result = "loss"
	elif away_score > home_score:
		home_result = "loss"
		away_result = "win"

	# Process stats for each player
	var stats_processed = 0
	for uid in player_stats:
		var stats = player_stats[uid]

		# Calculate player rating (0-10 scale)
		var player_team_result = home_result if stats.team_side == "home" else away_result
		stats.rating = _PlayerStatsProcessor.calculate_player_rating(stats, player_team_result)

		# Apply XP-based stat growth (for MVP player only in MVP mode)
		var is_mvp_player = PlayerData and PlayerData.has_method("get_uid") and PlayerData.get_uid() == uid
		if is_mvp_player:
			# Get events from result
			var events = result.get("events", [])

			# Convert events to XP
			var xp_bucket = _XpProcessor.convert_events_to_xp(events, uid, _last_match_setup)

			# Get overflow XP from PlayerData (if available)
			var xp_overflow = {}
			if PlayerData.has("xp_overflow"):
				xp_overflow = PlayerData.xp_overflow

			# Calculate growth
			var growth = _XpProcessor.calculate_growth(xp_bucket, PlayerData, xp_overflow)

			# Apply growth to PlayerData
			_XpProcessor.apply_growth_to_player(PlayerData, growth)

			print(
				(
					"[MatchSimulationManager] 📈 MVP player stat growth: %d total gains, %.1f total XP"
					% [growth.get("highlights", []).size(), growth.get("total_xp", 0.0)]
				)
			)

		# Update career stats
		var match_info = {
			"match_id": result.get("match_id", "match_%d" % Time.get_ticks_usec()),
			"opponent": match_data.get("opponent", "Unknown"),
			"home_away": "home" if stats.team_side == "home" else "away",
			"result": player_team_result,
			"goals_conceded": away_score if stats.team_side == "home" else home_score
		}

		career_stats_mgr.update_career_stats(uid, stats, match_info)
		stats_processed += 1

	# Save career stats database
	career_stats_mgr.save_to_database()

	print("[MatchSimulationManager] ✅ Player statistics processed for %d players" % stats_processed)


func _apply_match_rewards(result: Dictionary, _match_data: Dictionary) -> void:
	"""Apply XP and match rewards based on Phase F spec.

	Rewards:
	- Base XP: rating * 100
	- Result bonus: Win +300, Draw +100, Loss +0
	- Man of the Match (rating >= 8.5): +200
	"""
	var player_rating: float = float(result.get("player_rating", 5.0))
	var match_result_type: String = str(result.get("result", "draw"))

	# Base XP from rating
	var base_xp: int = int(round(player_rating * 100.0))

	# Result bonus
	var bonus_xp: int = 0
	match match_result_type:
		"win":
			bonus_xp += 300
		"draw":
			bonus_xp += 100

	# Man of the Match style bonus
	if player_rating >= 8.5:
		bonus_xp += 200

	var total_xp: int = max(base_xp + bonus_xp, 0)

	if has_node("/root/PlayerService"):
		var player_service: Node = get_node("/root/PlayerService")

		# Use a stable default player id for now
		var player_id := "user"

		# Check level before applying XP
		var before_profile: Dictionary = player_service.get_profile(player_id)
		var before_level: int = int(before_profile.get("level", 1))

		player_service.add_xp(player_id, total_xp)

		var after_profile: Dictionary = player_service.get_profile(player_id)
		var after_level: int = int(after_profile.get("level", before_level))
		var is_level_up: bool = after_level > before_level

		# Attach reward summary to result payload for UI consumers
		result["match_rewards"] = {
			"xp_gained": total_xp,
			"base_xp": base_xp,
			"bonus_xp": bonus_xp,
			"level_before": before_level,
			"level_after": after_level,
			"is_level_up": is_level_up
		}

		print(
			(
				"[MatchSimulationManager] ?? Match XP applied: +%d (rating: %.1f, result: %s, level %d → %d)"
				% [total_xp, player_rating, match_result_type, before_level, after_level]
			)
		)
	else:
		push_warning("[MatchSimulationManager] PlayerService not found, match XP not applied")


func _update_coach_trust(result: Dictionary) -> void:
	"""Update coach card trust based on match result

	Trust changes:
	- Win: +5 to all coaches
	- Draw: +1 to all coaches
	- Loss: -3 to all coaches
	"""
	var match_result_type: String = str(result.get("result", "draw"))

	var trust_change: int = 0
	if match_result_type == "win":
		trust_change = 5
	elif match_result_type == "draw":
		trust_change = 1
	elif match_result_type == "loss":
		trust_change = -3

	if trust_change != 0:
		# Apply trust change via CoachCardSystem
		if has_node("/root/CoachCardSystem"):
			var coach_system: Node = get_node("/root/CoachCardSystem")
			coach_system.increase_trust_all(trust_change)

			print(
				"[MatchSimulationManager] 👔 Coach trust updated: %+d (result: %s)" % [trust_change, match_result_type]
			)
		else:
			push_warning("[MatchSimulationManager] CoachCardSystem not found, trust not updated")

		# Apply win/loss condition effects
		if has_node("/root/ConditionSystem"):
			var condition_system = get_node("/root/ConditionSystem")

			if match_result_type == "win":
				condition_system.apply_victory_bonus()
				print("[MatchSimulationManager] 🏆 Win - Victory bonus applied")
			elif match_result_type == "loss":
				condition_system.apply_defeat_penalty()
				print("[MatchSimulationManager] 😞 Loss - Defeat penalty applied")


# ============================================================================
# HELPER FUNCTIONS
# ============================================================================

const MATCH_POSITION_TEMPLATE := [
	"GK", "RB", "CB", "CB", "LB", "CDM", "CM", "CAM", "RW", "LW", "ST", "ST", "CB", "CM", "LW", "RW", "GK", "ST"
]

const POSITION_PATTERNS := {
	"GK": ["GK", "GOALKEEPER", "KEEPER"],
	"CB": ["CB", "CENTRE BACK", "CENTER BACK", "DEFENDER", "DC"],
	"RB": ["RB", "RIGHT BACK", "DR"],
	"LB": ["LB", "LEFT BACK", "DL"],
	"RWB": ["RWB", "RIGHT WING BACK"],
	"LWB": ["LWB", "LEFT WING BACK"],
	"CDM": ["CDM", "DM", "DEFENSIVE MID"],
	"CM": ["CM", "MIDFIELDER", "MF"],
	"CAM": ["CAM", "AM", "ATTACKING MID"],
	"RM": ["RM", "RIGHT MID", "MR"],
	"LM": ["LM", "LEFT MID", "ML"],
	"LW": ["LW", "LEFT WING", "AML"],
	"RW": ["RW", "RIGHT WING", "AMR"],
	"ST": ["ST", "STRIKER", "CF", "FORWARD", "F"]
}


func _get_player_data() -> Dictionary:
	"""Get current player data"""
	if has_node("/root/PlayerData"):
		var player_data_node = get_node("/root/PlayerData")
		if player_data_node.has_method("get_player_data"):
			var data: Dictionary = player_data_node.get_player_data().duplicate(true)

			# Ensure critical fields for OpenFootball conversion
			if not data.has("ca") or int(data.get("ca", 0)) <= 0:
				data["ca"] = int(data.get("overall", 60))
			data["pa"] = 199
			if not data.has("uid"):
				data["uid"] = 1
			if not data.has("player_id"):
				data["player_id"] = "academy_main"

			if not data.has("stamina"):
				data["stamina"] = 100
			if not data.has("form"):
				data["form"] = 50
			if not data.has("morale"):
				data["morale"] = 50

			# Flatten nested attribute groups if present
			var flat_attrs: Dictionary = {}
			if data.has("technical") and data.technical is Dictionary:
				for attr_name in data.technical.keys():
					flat_attrs[attr_name] = data.technical[attr_name]
			if data.has("mental") and data.mental is Dictionary:
				for attr_name in data.mental.keys():
					flat_attrs[attr_name] = data.mental[attr_name]
			if data.has("physical") and data.physical is Dictionary:
				for attr_name in data.physical.keys():
					flat_attrs[attr_name] = data.physical[attr_name]
			if data.has("goalkeeper") and data.goalkeeper is Dictionary:
				for attr_name in data.goalkeeper.keys():
					flat_attrs[attr_name] = data.goalkeeper[attr_name]

			if not flat_attrs.is_empty():
				data["attributes"] = _build_openfootball_attribute_map(data, flat_attrs)

			if not data.has("overall"):
				var total: int = 0
				var count: int = 0
				for value in flat_attrs.values():
					var numeric_value: int = int(value)
					total += numeric_value
					count += 1
				if count > 0:
					data["overall"] = int(total / count)

			return data

	push_warning("[MatchSimulationManager] PlayerData not available")
	return {}


func _get_player_condition() -> float:
	"""Get current player condition/form"""
	if has_node("/root/ConditionSystem"):
		var condition_system = get_node("/root/ConditionSystem")
		if condition_system and condition_system.has_method("get_condition_percentage"):
			var pct := float(condition_system.get_condition_percentage())
			return clampf(pct / 100.0, 0.0, 1.0)
	return -1.0


func _build_openfootball_attribute_map(player_data: Dictionary, flat_attrs: Dictionary) -> Dictionary:
	var attrs: Dictionary = {}

	var technical: Dictionary = {}
	var technical_raw: Variant = player_data.get("technical")
	if technical_raw is Dictionary:
		technical = technical_raw
	attrs["corners"] = int(technical.get("corners", flat_attrs.get("corners", 50)))
	attrs["crossing"] = int(technical.get("crossing", flat_attrs.get("crossing", 50)))
	attrs["dribbling"] = int(technical.get("dribbling", flat_attrs.get("dribbling", 50)))
	attrs["finishing"] = int(technical.get("finishing", flat_attrs.get("finishing", 50)))
	attrs["first_touch"] = int(technical.get("first_touch", flat_attrs.get("first_touch", 50)))
	attrs["free_kicks"] = int(technical.get("free_kicks", flat_attrs.get("free_kicks", 50)))
	attrs["heading"] = int(technical.get("heading", flat_attrs.get("heading", 50)))
	attrs["long_shots"] = int(technical.get("long_shots", flat_attrs.get("long_shots", 50)))
	attrs["long_throws"] = int(
		technical.get("long_throws", flat_attrs.get("long_throws", flat_attrs.get("throw_ins", 50)))
	)
	attrs["marking"] = int(technical.get("marking", flat_attrs.get("marking", 50)))
	attrs["passing"] = int(technical.get("passing", flat_attrs.get("passing", 50)))
	attrs["penalty_taking"] = int(
		technical.get("penalty_taking", flat_attrs.get("penalty_taking", flat_attrs.get("penalty_kicks", 50)))
	)
	attrs["tackling"] = int(technical.get("tackling", flat_attrs.get("tackling", 50)))
	attrs["technique"] = int(technical.get("technique", flat_attrs.get("technique", 50)))

	var mental: Dictionary = {}
	var mental_raw: Variant = player_data.get("mental")
	if mental_raw is Dictionary:
		mental = mental_raw
	attrs["aggression"] = int(mental.get("aggression", flat_attrs.get("aggression", 50)))
	attrs["anticipation"] = int(mental.get("anticipation", flat_attrs.get("anticipation", 50)))
	attrs["bravery"] = int(mental.get("bravery", flat_attrs.get("bravery", 50)))
	attrs["composure"] = int(mental.get("composure", flat_attrs.get("composure", 50)))
	attrs["concentration"] = int(mental.get("concentration", flat_attrs.get("concentration", 50)))
	attrs["decisions"] = int(mental.get("decisions", flat_attrs.get("decisions", 50)))
	attrs["determination"] = int(mental.get("determination", flat_attrs.get("determination", 50)))
	attrs["flair"] = int(mental.get("flair", flat_attrs.get("flair", 50)))
	attrs["leadership"] = int(mental.get("leadership", flat_attrs.get("leadership", 50)))
	attrs["off_the_ball"] = int(mental.get("off_the_ball", flat_attrs.get("off_the_ball", 50)))
	attrs["positioning"] = int(mental.get("positioning", flat_attrs.get("positioning", 50)))
	attrs["teamwork"] = int(mental.get("teamwork", flat_attrs.get("teamwork", 50)))
	attrs["vision"] = int(mental.get("vision", flat_attrs.get("vision", 50)))
	attrs["work_rate"] = int(mental.get("work_rate", flat_attrs.get("work_rate", 50)))

	var physical: Dictionary = {}
	var physical_raw: Variant = player_data.get("physical")
	if physical_raw is Dictionary:
		physical = physical_raw
	attrs["acceleration"] = int(physical.get("acceleration", flat_attrs.get("acceleration", 50)))
	attrs["agility"] = int(physical.get("agility", flat_attrs.get("agility", 50)))
	attrs["balance"] = int(physical.get("balance", flat_attrs.get("balance", 50)))
	attrs["jumping"] = int(physical.get("jumping", flat_attrs.get("jumping", 50)))
	attrs["natural_fitness"] = int(physical.get("natural_fitness", flat_attrs.get("natural_fitness", 50)))
	attrs["pace"] = int(physical.get("pace", flat_attrs.get("pace", flat_attrs.get("speed", 50))))
	attrs["stamina"] = int(physical.get("stamina", flat_attrs.get("stamina", 50)))
	attrs["strength"] = int(physical.get("strength", flat_attrs.get("strength", 50)))

	var goalkeeper: Dictionary = {}
	var goalkeeper_raw: Variant = player_data.get("goalkeeper")
	if goalkeeper_raw is Dictionary:
		goalkeeper = goalkeeper_raw
	attrs["aerial_ability"] = int(
		goalkeeper.get("aerial_ability", flat_attrs.get("aerial_ability", flat_attrs.get("aerial_reach", 50)))
	)
	attrs["command_of_area"] = int(goalkeeper.get("command_of_area", flat_attrs.get("command_of_area", 50)))
	attrs["communication"] = int(goalkeeper.get("communication", flat_attrs.get("communication", 50)))
	attrs["handling"] = int(goalkeeper.get("handling", flat_attrs.get("handling", 50)))
	attrs["kicking"] = int(goalkeeper.get("kicking", flat_attrs.get("kicking", 50)))
	attrs["reflexes"] = int(goalkeeper.get("reflexes", flat_attrs.get("reflexes", 50)))

	return attrs


func _canonicalize_position(raw_value, slot_index: int) -> String:
	var fallback: String = String(MATCH_POSITION_TEMPLATE[min(slot_index, MATCH_POSITION_TEMPLATE.size() - 1)])
	var source_value: String = String(raw_value)
	var upper: String = source_value.strip_edges().to_upper()
	if upper.is_empty():
		return fallback
	for canonical in POSITION_PATTERNS.keys():
		for token in POSITION_PATTERNS[canonical]:
			if upper.find(token) != -1:
				return canonical
	var simple: String = upper.strip_edges()
	if simple.length() <= 3:
		return simple
	return fallback


func _build_match_roster(main_player: Dictionary) -> Array:
	var roster: Array = []
	var used_ids: Dictionary = {}

	var prepared_main: Dictionary = _prepare_main_player_for_match(main_player)
	var main_key: String = str(
		prepared_main.get("player_id", prepared_main.get("uid", prepared_main.get("name", "main")))
	)
	used_ids[main_key] = true
	roster.append(prepared_main)

	var desired_total: int = 18

	# Priority 1: Use MyTeamManager.first_team (has correct positions from StageManager)
	if has_node("/root/MyTeamManager"):
		var my_team_mgr = get_node("/root/MyTeamManager")
		if my_team_mgr.first_team.size() > 0:
			print("[MatchSim] Using MyTeamManager.first_team (%d players)" % my_team_mgr.first_team.size())
			for player_data in my_team_mgr.first_team:
				if roster.size() >= desired_total:
					break
				if player_data.get("name", "") == prepared_main.get("name", ""):
					continue
				var converted: Dictionary = _convert_saved_player_to_match_player(player_data, roster.size())
				if converted.is_empty():
					continue
				var key: String = str(converted.get("player_id", converted.get("uid", converted.get("name", ""))))
				if used_ids.has(key):
					continue
				roster.append(converted)
				used_ids[key] = true

	# Priority 2: Fallback to MyTeamData if still need more players
	if roster.size() < desired_total and has_node("/root/MyTeamData"):
		var my_team = get_node("/root/MyTeamData")
		var candidate_ids: Array = []
		if my_team.current_team.has("players") and my_team.current_team.players.size() > 0:
			candidate_ids = my_team.current_team.players.duplicate()
		else:
			for saved_player in my_team.saved_players:
				candidate_ids.append(saved_player.get("id", ""))

		for saved_id in candidate_ids:
			if roster.size() >= desired_total:
				break
			if saved_id == "":
				continue

			var saved_data: Dictionary = {}
			if my_team.has_method("get_player_by_id"):
				saved_data = my_team.get_player_by_id(saved_id)
			if saved_data.is_empty():
				saved_data = _find_saved_player_by_id(my_team, saved_id)
			if saved_data.is_empty():
				continue

			if saved_data.get("name", "") == prepared_main.get("name", ""):
				continue

			var converted: Dictionary = _convert_saved_player_to_match_player(saved_data, roster.size())
			if converted.is_empty():
				continue
			var key: String = str(converted.get("player_id", converted.get("uid", converted.get("name", ""))))
			if used_ids.has(key):
				continue
			roster.append(converted)
			used_ids[key] = true
		# Fallback: fill remaining slots from saved_players list
		if roster.size() < desired_total:
			for saved_player in my_team.saved_players:
				if roster.size() >= desired_total:
					break
				if saved_player.get("name", "") == prepared_main.get("name", ""):
					continue
				var converted_fallback: Dictionary = _convert_saved_player_to_match_player(saved_player, roster.size())
				if converted_fallback.is_empty():
					continue
				var fallback_key: String = str(
					converted_fallback.get(
						"player_id", converted_fallback.get("uid", converted_fallback.get("name", ""))
					)
				)
				if used_ids.has(fallback_key):
					continue
				roster.append(converted_fallback)
				used_ids[fallback_key] = true

	# If still short, generate placeholder youth players
	while roster.size() < desired_total:
		var slot_index: int = roster.size()
		var position: String = MATCH_POSITION_TEMPLATE[min(slot_index, MATCH_POSITION_TEMPLATE.size() - 1)]
		var placeholder: Dictionary = _generate_placeholder_player(slot_index, position)
		var placeholder_key: String = str(
			placeholder.get("player_id", placeholder.get("uid", placeholder.get("name", "placeholder")))
		)
		if used_ids.has(placeholder_key):
			continue
		used_ids[placeholder_key] = true
		roster.append(placeholder)

	# Sort roster by position to ensure proper starting 11 for 4-4-2 formation
	# Priority: GK → Defenders → Midfielders → Forwards → Subs
	roster = _sort_roster_by_formation(roster)

	return roster


func _prepare_main_player_for_match(main_player: Dictionary) -> Dictionary:
	var prepared: Dictionary = main_player.duplicate(true)
	if not prepared.has("player_id"):
		prepared["player_id"] = "academy_main"
	if not prepared.has("uid"):
		prepared["uid"] = 1
	prepared["uid"] = int(prepared.get("uid", 1))
	prepared["ca"] = int(prepared.get("ca", prepared.get("overall", 60)))
	prepared["pa"] = int(prepared.get("pa", min(prepared["ca"] + 20, 199)))
	var system_condition := _get_player_condition()
	if system_condition >= 0.0:
		prepared["condition"] = system_condition
	else:
		prepared["condition"] = _normalize_match_condition(prepared.get("condition", 4))
	prepared["form"] = int(prepared.get("form", 50))
	prepared["morale"] = int(prepared.get("morale", 50))
	prepared["stamina"] = int(prepared.get("stamina", 100))
	prepared["fatigue"] = float(prepared.get("fatigue", 0.0))
	if not prepared.has("position_category"):
		prepared["position_category"] = _infer_position_category(prepared.get("position", "M"))
	prepared["position"] = _canonicalize_position(prepared.get("position", ""), 0)
	prepared["position_category"] = _infer_position_category(prepared["position"])
	_ensure_player_attribute_map(prepared)
	return prepared


func _convert_saved_player_to_match_player(saved_player: Dictionary, slot_index: int) -> Dictionary:
	if saved_player.is_empty():
		return {}

	var player_id: String = String(saved_player.get("id", "saved_player_%d" % slot_index))
	var uid: int = int(hash(player_id) & 0x7fffffff)
	uid = uid % 1000000000
	if uid == 0:
		uid = 100000 + slot_index

	# Support both "ca" (MyTeamManager) and "overall" (MyTeamData) field names
	var ca: int = int(saved_player.get("ca", saved_player.get("overall", 60)))
	ca = clampi(ca, 1, 200)
	var pa: int = int(saved_player.get("pa", saved_player.get("potential", ca + 20)))
	pa = clampi(pa, ca, 199)

	var canonical_position: String = _canonicalize_position(saved_player.get("position", ""), slot_index)

	var player_data: Dictionary = {}
	player_data["name"] = saved_player.get("name", "Youth Player %d" % slot_index)
	player_data["player_id"] = player_id
	player_data["uid"] = uid
	player_data["position"] = canonical_position
	player_data["position_category"] = saved_player.get(
		"position_category", _infer_position_category(canonical_position)
	)
	player_data["age"] = int(saved_player.get("age", 18))
	player_data["technical"] = saved_player.get("technical", {})
	player_data["mental"] = saved_player.get("mental", {})
	player_data["physical"] = saved_player.get("physical", {})
	player_data["goalkeeper"] = saved_player.get("goalkeeper", {})
	player_data["ca"] = ca
	player_data["pa"] = pa
	player_data["overall"] = saved_player.get("overall", ca)
	player_data["condition"] = _normalize_match_condition(saved_player.get("condition", 4))
	player_data["form"] = int(saved_player.get("form", 50))
	player_data["morale"] = int(saved_player.get("morale", 50))
	player_data["stamina"] = int(saved_player.get("stamina", 100))
	player_data["fatigue"] = float(saved_player.get("fatigue", 0.0))
	player_data["personality"] = saved_player.get("personality", {})
	# Valid archetypes: Leader, Genius, Workhorse, Rebel, Steady (match Rust enum)
	player_data["personality_archetype"] = saved_player.get("personality_archetype", "Steady")

	_ensure_player_attribute_map(player_data)
	return player_data


func _ensure_player_attribute_map(player_data: Dictionary) -> void:
	if player_data.has("attributes") and player_data.attributes is Dictionary and not player_data.attributes.is_empty():
		return

	var flat_attrs: Dictionary = {}

	if player_data.has("technical") and player_data.technical is Dictionary:
		for attr_name in player_data.technical.keys():
			flat_attrs[attr_name] = int(player_data.technical[attr_name])
	else:
		player_data["technical"] = {}

	if player_data.has("mental") and player_data.mental is Dictionary:
		for attr_name in player_data.mental.keys():
			flat_attrs[attr_name] = int(player_data.mental[attr_name])
	else:
		player_data["mental"] = {}

	if player_data.has("physical") and player_data.physical is Dictionary:
		for attr_name in player_data.physical.keys():
			flat_attrs[attr_name] = int(player_data.physical[attr_name])
	else:
		player_data["physical"] = {}

	if player_data.has("goalkeeper") and player_data.goalkeeper is Dictionary:
		for attr_name in player_data.goalkeeper.keys():
			flat_attrs[attr_name] = int(player_data.goalkeeper[attr_name])
	else:
		player_data["goalkeeper"] = {}

	player_data["attributes"] = _build_openfootball_attribute_map(player_data, flat_attrs)


# ============================================================================
# DEPRECATED: The following functions are no longer used (Phase B)
# Opponent roster generation is now handled by OpponentRosterProvider autoload
# These functions are kept for reference but will be removed in future cleanup
# ============================================================================


func _build_opponent_roster_from_csv() -> void:
	# DEPRECATED: Use OpponentRosterProvider.get_opponent_roster() instead
	print("[MatchSimulationManager] ⚠️ DEPRECATED: _build_opponent_roster_from_csv() called")
	var use_real_names := _use_real_names_for_opponents()

	# ========================================================================
	# Phase 1: Check static cache first
	# ========================================================================
	if _csv_cache_initialized and not _csv_parsed_teams.is_empty():
		print("[MatchSimulationManager] ✅ Using cached CSV data (%d teams)" % _csv_parsed_teams.size())
		_load_opponent_from_cache()
		return

	# ========================================================================
	# Phase 1: Cache miss - parse CSV and populate cache
	# ========================================================================
	print("[MatchSimulationManager] 🔍 DEBUG: CSV path: %s" % OPPONENT_CSV_PATH)
	var file := FileAccess.open(OPPONENT_CSV_PATH, FileAccess.READ)
	if file == null:
		push_warning("[MatchSimulationManager] ❌ Unable to open opponent dataset at %s" % OPPONENT_CSV_PATH)
		push_warning("[MatchSimulationManager] ❌ FileAccess error: %s" % str(FileAccess.get_open_error()))
		return
	print("[MatchSimulationManager] ✅ CSV file opened successfully")

	var headers: PackedStringArray = file.get_csv_line()
	if headers.is_empty():
		push_warning("[MatchSimulationManager] Opponent dataset is empty")
		return

	var team_map: Dictionary = {}

	while not file.eof_reached():
		var row: PackedStringArray = file.get_csv_line()
		if row.is_empty():
			continue

		var data: Dictionary = {}
		for i in range(headers.size()):
			var key := headers[i]
			if i < row.size():
				data[key] = row[i]
			else:
				data[key] = ""

		var pseudo_team: String = String(data.get("PseudoTeam", "")).strip_edges()
		if pseudo_team == "":
			continue

		if not team_map.has(pseudo_team):
			team_map[pseudo_team] = []
		var team_rows_variant: Variant = team_map[pseudo_team]
		var team_rows: Array = []
		if team_rows_variant is Array:
			team_rows = team_rows_variant
		team_rows.append(data)
		team_map[pseudo_team] = team_rows

	var best_team: String = ""
	var best_avg: float = 99999.0

	print("[MatchSimulationManager] 🔍 DEBUG: Parsed %d teams from CSV" % team_map.size())
	var teams_with_18_plus := 0

	for team in team_map.keys():
		var rows_variant: Variant = team_map[team]
		var rows: Array = []
		if rows_variant is Array:
			rows = rows_variant
		if rows.size() < 18:
			continue
		teams_with_18_plus += 1
		var cas: Array = []
		for entry in rows:
			var ca_value := _parse_float(entry.get("CA", "0"))
			if ca_value <= 0.0:
				continue
			cas.append(ca_value)
		if cas.size() < 18:
			continue
		cas.sort()
		var avg: float = 0.0
		for i in range(18):
			avg += cas[i]
		avg /= 18.0
		if avg < best_avg:
			best_avg = avg
			best_team = team

	print("[MatchSimulationManager] 🔍 DEBUG: Found %d teams with 18+ players" % teams_with_18_plus)
	print("[MatchSimulationManager] 🔍 DEBUG: Best team: '%s' (avg CA: %.1f)" % [best_team, best_avg])

	if best_team == "":
		push_warning("[MatchSimulationManager] ❌ No suitable opponent team found in dataset")
		return

	var selected_rows_variant: Variant = team_map[best_team]
	var selected_rows: Array = []
	if selected_rows_variant is Array:
		selected_rows = selected_rows_variant
	selected_rows.sort_custom(func(a, b): return _parse_float(a.get("CA", "0")) < _parse_float(b.get("CA", "0")))

	_cached_opponent_roster.clear()
	_opponent_team_name = best_team
	if use_real_names and not selected_rows.is_empty():
		var real_team_name: String = String(selected_rows[0].get("Team", best_team)).strip_edges()
		if real_team_name != "":
			_opponent_team_name = real_team_name

	for slot_index in range(min(18, selected_rows.size())):
		var csv_row: Dictionary = selected_rows[slot_index]
		var player: Dictionary = _create_opponent_player_from_row(csv_row, slot_index, use_real_names)
		_cached_opponent_roster.append(player)

	print("[MatchSimulationManager] ✅ Loaded %d players from CSV into cache" % _cached_opponent_roster.size())
	print("[MatchSimulationManager] ✅ Opponent team: '%s'" % _opponent_team_name)

	# ========================================================================
	# Phase 1: Save all teams to static cache for future matches
	# ========================================================================
	if not _csv_cache_initialized:
		_populate_csv_cache(team_map, use_real_names)
		_csv_cache_initialized = true
		print("[MatchSimulationManager] ✅ CSV cache populated with %d teams" % _csv_parsed_teams.size())


func _create_opponent_player_from_row(csv_row: Dictionary, slot_index: int, use_real_names: bool) -> Dictionary:
	var raw_position: String = String(csv_row.get("Position", "M"))
	var canonical_position: String = _canonicalize_position(raw_position, slot_index)

	var player := _generate_placeholder_player(slot_index, canonical_position)

	var real_name: String = String(csv_row.get("Name", "Opponent %02d" % slot_index)).strip_edges()
	var pseudo_name: String = String(csv_row.get("PseudoName", real_name)).strip_edges()
	var chosen_name := real_name if use_real_names else pseudo_name
	if chosen_name == "":
		chosen_name = real_name if real_name != "" else pseudo_name
	if chosen_name == "":
		chosen_name = "Opponent %02d" % slot_index
	player["name"] = chosen_name

	var real_team: String = String(csv_row.get("Team", "Opponent")).strip_edges()
	var pseudo_team: String = String(csv_row.get("PseudoTeam", real_team)).strip_edges()
	var chosen_team := real_team if use_real_names else pseudo_team
	if chosen_team == "":
		chosen_team = real_team if real_team != "" else pseudo_team
	if chosen_team == "":
		chosen_team = "Opponent"

	player["player_id"] = "%s_%d" % [chosen_team.to_lower().replace(" ", "_"), slot_index]
	player["uid"] = abs(hash(player["player_id"])) % 1000000000
	player["team"] = chosen_team
	player["team_name"] = chosen_team
	player["position"] = canonical_position
	player["position_category"] = _infer_position_category(canonical_position)

	var ca_value := int(round(_parse_float(csv_row.get("CA", str(player["ca"])))))
	player["ca"] = clampi(ca_value, 35, 120)
	player["overall"] = player["ca"]

	var pa_value: int = int(round(_parse_float(csv_row.get("PA", str(player["pa"])))))
	player["pa"] = clampi(pa_value, player["ca"] + 2, 199)

	var age_value: int = int(round(_parse_float(csv_row.get("Age", "18"))))
	player["age"] = clampi(age_value, 16, 35)
	player["condition"] = _condition_from_age(player["age"])

	player["fatigue"] = 0.0
	player["form"] = 50
	player["morale"] = 50

	# Parts 기반 랜덤 외형 생성
	player["appearance"] = _PlayerAppearanceBridge.create_random()

	return player


func _create_opponent_player_from_cache_entry(
	entry: Dictionary, slot_index: int, opponent_name: String, target_ca: int, use_real_names: bool
) -> Dictionary:
	var raw_position: String = String(entry.get("position", entry.get("primary_position", "M")))
	var canonical_position: String = _canonicalize_position(raw_position, slot_index)

	var player := _generate_placeholder_player(slot_index, canonical_position)

	var display_name: String = String(
		entry.get("display_name", entry.get("name", entry.get("pseudo_name", player["name"])))
	)
	if display_name.is_empty():
		display_name = "Opponent %02d" % slot_index
	player["name"] = display_name

	var base_team: String = ""
	if use_real_names:
		base_team = String(entry.get("display_team", entry.get("team", opponent_name))).strip_edges()
	else:
		base_team = String(entry.get("display_team", entry.get("pseudo_team", opponent_name))).strip_edges()
	if base_team.is_empty():
		base_team = opponent_name

	var uid_value: int = int(entry.get("uid", 900000 + slot_index))
	player["player_id"] = "%s_%02d" % [base_team.to_lower().replace(" ", "_"), slot_index + 1]
	player["uid"] = uid_value if uid_value > 0 else abs(hash(player["player_id"])) % 1000000000
	player["team"] = base_team
	player["team_name"] = base_team

	player["position"] = canonical_position
	player["position_category"] = _infer_position_category(canonical_position)

	var ca_value: int = int(entry.get("ca", target_ca))
	if ca_value <= 0:
		ca_value = target_ca
	player["ca"] = clampi(ca_value, 40, 185)
	player["overall"] = player["ca"]

	var pa_value: int = int(entry.get("pa", player["ca"] + 12))
	if pa_value <= player["ca"]:
		pa_value = player["ca"] + 12
	player["pa"] = clampi(pa_value, player["ca"] + 6, 199)

	var age_value: int = int(entry.get("age", 21))
	player["age"] = clampi(age_value, 16, 34)
	player["condition"] = _condition_from_age(player["age"])

	player["fatigue"] = 0.0
	player["form"] = 55
	player["morale"] = 55
	player["stamina"] = 100

	# Parts 기반 랜덤 외형 생성
	player["appearance"] = _PlayerAppearanceBridge.create_random()

	_ensure_player_attribute_map(player)
	return player


func _condition_from_age(age: int) -> float:
	var normalized_age: int = clampi(age, 16, 35)
	var base: float = 1.0 - float(normalized_age - 16) / 35.0
	return clampf(base, 0.6, 0.95)


func _parse_float(value) -> float:
	if typeof(value) == TYPE_FLOAT:
		return float(value)
	if typeof(value) == TYPE_INT:
		return float(value)
	var str_value: String = String(value).strip_edges()
	if str_value.is_valid_float():
		return str_value.to_float()
	return 0.0


# ============================================================================
# MATCH SCREEN IMPROVED HELPER FUNCTIONS (Moved from MatchScreenImproved.gd)
# ============================================================================


func _get_team_data(team_type: String) -> Dictionary:
	"""팀 데이터 구성 (MyTeamManager 연동)"""
	if team_type == "home":
		# 실제 우리 팀 데이터 사용
		if MyTeamManager and MyTeamManager.can_start_match():
			return MyTeamManager.get_match_team_data()
		else:
			print("[MatchSimulationManager] 팀 데이터 불충분, 기본 팀 사용")
			return _get_default_team_data("home")
	else:
		# AI 상대팀 생성 (난이도 설정 가능)
		var difficulty = "normal"  # 나중에 GameManager에서 가져올 수 있음
		if MyTeamManager:
			return MyTeamManager.generate_ai_opponent(difficulty)
		else:
			return _get_default_team_data("away")


func _get_default_team_data(team_type: String) -> Dictionary:
	"""기본 팀 데이터 (폴백용)"""
	var positions = ["GK", "DF", "DF", "DF", "DF", "MF", "MF", "MF", "FW", "FW", "FW"]
	var players = []
	var base_ca = 60 if team_type == "home" else 55

	for i in range(11):
		players.append(
			{
				"name": "%s Player %d" % [team_type.capitalize(), i + 1],
				"position": positions[i],
				"ca": base_ca + randi_range(-5, 5)
			}
		)

	return {"name": "Default %s Team" % team_type.capitalize(), "players": players}


func _normalize_openfootball_result(
	raw_result: Dictionary, home_team: Dictionary = {}, away_team: Dictionary = {}
) -> Dictionary:
	var payload: Dictionary = raw_result.get("response", raw_result.duplicate(true))

	var error_value = raw_result.get("error", payload.get("error", null))
	if typeof(error_value) == TYPE_STRING:
		var trimmed: String = str(error_value).strip_edges()
		if trimmed != "":
			return {"error": trimmed}
	elif error_value is bool and error_value:
		return {"error": "match_simulation_failed"}

	var success_value = raw_result.get("success", payload.get("success", true))
	if success_value is bool and not success_value:
		return {"error": str(payload.get("message", raw_result.get("message", "Unknown match error")))}

	var timeline_events: Array = payload.get("events", [])
	var legacy_doc_key := "re" + "play" + "_doc"
	var legacy_root_key := "re" + "play"
	if timeline_events.is_empty():
		var doc_variant: Variant = payload.get("timeline_doc", payload.get(legacy_doc_key, {}))
		if doc_variant is Dictionary:
			timeline_events = (doc_variant as Dictionary).get("events", [])
	if timeline_events.is_empty():
		var raw_resp: Dictionary = payload.get("raw_result", raw_result.get("raw_result", {}))
		if raw_resp is Dictionary:
			var legacy_block: Variant = raw_resp.get(legacy_root_key, {})
			if legacy_block is Dictionary:
				timeline_events = (legacy_block as Dictionary).get("events", [])

	var rosters: Dictionary = payload.get("rosters", {})
	if rosters.is_empty():
		var raw_rosters = payload.get("raw_result", {}).get("rosters", {})
		if raw_rosters is Dictionary:
			rosters = raw_rosters.duplicate(true)
	if rosters.is_empty():
		var inferred_rosters: Dictionary = {}
		if home_team:
			inferred_rosters["home"] = home_team.duplicate(true)
		if away_team:
			inferred_rosters["away"] = away_team.duplicate(true)
		if not inferred_rosters.is_empty():
			rosters = inferred_rosters

	var match_result: Dictionary = payload.get("match_result", {})
	if match_result.is_empty():
		var raw_match_result = payload.get("raw_result", {}).get("match_result", {})
		if raw_match_result is Dictionary:
			match_result = raw_match_result.duplicate(true)
	if match_result.is_empty():
		match_result = {}
	match_result["goals_home"] = int(match_result.get("goals_home", payload.get("score_home", 0)))
	match_result["goals_away"] = int(match_result.get("goals_away", payload.get("score_away", 0)))

	var normalized_events := _normalize_openfootball_events(timeline_events)

	var timeline_doc := {
		"events": normalized_events,
		"rosters": rosters,
		"metadata": payload.get("metadata", payload.get("raw_result", {}).get("metadata", {})),
		"seed": payload.get("seed", raw_result.get("seed", 0))
	}

	return {
		"timeline_doc": timeline_doc,
		"events": normalized_events,
		"rosters": rosters,
		"match_result": match_result,
		"raw_result": payload.get("raw_result", raw_result.duplicate(true)),
		"metadata": timeline_doc.metadata,
		"seed": timeline_doc.seed
	}


func _normalize_openfootball_events(events: Array) -> Array:
	var normalized: Array = []
	for event in events:
		if not (event is Dictionary):
			continue
		var clone: Dictionary = (event as Dictionary).duplicate(true)

		var type_value: String = String(clone.get("kind", clone.get("type", "")))
		if type_value == "":
			type_value = "unknown"
		clone["type_label"] = type_value
		var normalized_type: String = type_value.to_lower()
		clone["type"] = normalized_type
		clone["type_key"] = normalized_type.replace("_", "")

		var base = clone.get("base", {})
		var timestamp_ms := int(clone.get("timestamp", 0))
		if base is Dictionary:
			if not clone.has("team_id"):
				clone["team_id"] = int(base.get("team_id", clone.get("team_id", -1)))
			else:
				clone["team_id"] = int(clone.get("team_id"))
			if not clone.has("minute"):
				clone["minute"] = float(base.get("t", base.get("minute", 0)))
			else:
				clone["minute"] = float(clone.get("minute"))
			var time_seconds := float(base.get("t", clone.get("minute", 0.0)))
			clone["time"] = time_seconds
			timestamp_ms = int(round(time_seconds * 1000.0))
			if not clone.has("player_id") and base.has("player_id"):
				clone["player_id"] = base.get("player_id")
		else:
			var minute_value := float(clone.get("minute", clone.get("time", 0.0)))
			clone["minute"] = minute_value
			var derived_time := float(clone.get("time", minute_value))
			clone["time"] = derived_time
			timestamp_ms = int(round(derived_time * 1000.0))
			if clone.has("team_id"):
				clone["team_id"] = int(clone["team_id"])
		clone["timestamp"] = timestamp_ms

		# ST-005 Phase 1: Use EventParser for event augmentation
		match normalized_type:
			"pass":
				_event_parser.parse_pass_event(clone)
			"through_ball":
				_event_parser.parse_throughball_event(clone)
			"shot":
				_event_parser.parse_shot_event(clone)
			"run":
				_event_parser.parse_run_event(clone)
			"dribble":
				_event_parser.parse_dribble_event(clone)
			"communication":
				_event_parser.parse_communication_event(clone)
			"header":
				_event_parser.parse_header_event(clone)
			"boundary":
				_event_parser.parse_boundary_event(clone)

		normalized.append(clone)

		if normalized_type == "shot":
			var outcome_str := str(clone.get("outcome", clone.get("result", ""))).to_lower()
			if outcome_str == "goal":
				var goal_event: Dictionary = clone.duplicate(true)
				goal_event["kind"] = "goal"
				goal_event["type"] = "goal"
				goal_event["type_label"] = "goal"
				goal_event["type_key"] = "goal"
				goal_event["source_kind"] = normalized_type
				goal_event["is_goal_event"] = true

				if not goal_event.has("position"):
					if clone.has("target") and clone.get("target") is Dictionary:
						goal_event["position"] = clone.get("target")
					elif clone.has("at") and clone.get("at") is Dictionary:
						goal_event["position"] = clone.get("at")
					elif base is Dictionary and base.has("pos") and base.get("pos") is Dictionary:
						goal_event["position"] = base.get("pos")

				normalized.append(goal_event)
	return normalized


# ============================================================================
# Phase 1: CSV Cache Helper Functions
# ============================================================================


func _populate_csv_cache(team_map: Dictionary, use_real_names: bool) -> void:
	"""DEPRECATED: Moved to OpponentRosterProvider._initialize_csv_cache()"""
	_csv_parsed_teams.clear()

	for team_name in team_map.keys():
		var rows_variant: Variant = team_map[team_name]
		var rows: Array = []
		if rows_variant is Array:
			rows = rows_variant
		if rows.size() < 11:  # Need at least 11 players
			continue

		# Convert CSV rows to player dictionaries
		var team_players: Array = []
		for i in range(min(22, rows.size())):  # Store up to 22 players per team
			var csv_row: Dictionary = rows[i]
			var player: Dictionary = _create_opponent_player_from_row(csv_row, i, use_real_names)
			team_players.append(player)

		# Store with both pseudo and real names as keys
		var pseudo_team: String = String(team_name).strip_edges()
		_csv_parsed_teams[pseudo_team] = team_players

		# Also store by real team name if different
		if not rows.is_empty() and use_real_names:
			var real_team: String = String(rows[0].get("Team", "")).strip_edges()
			if real_team != "" and real_team != pseudo_team:
				_csv_parsed_teams[real_team] = team_players


func _load_opponent_from_cache() -> void:
	"""DEPRECATED: Moved to OpponentRosterProvider._get_roster_from_csv()"""
	if _csv_parsed_teams.is_empty():
		push_warning("[MatchSimulationManager] ❌ Cache is empty")
		return

	# Simple strategy: pick a random team from cache
	# In future, can add CA-based selection
	var team_names: Array = _csv_parsed_teams.keys()
	var random_team: String = team_names[randi() % team_names.size()]

	var cached_players: Array = _csv_parsed_teams[random_team]

	_cached_opponent_roster.clear()
	_opponent_team_name = random_team

	# Copy up to 18 players
	for i in range(min(18, cached_players.size())):
		_cached_opponent_roster.append(cached_players[i].duplicate(true))

	print(
		(
			"[MatchSimulationManager] ✅ Loaded %d players from cache (team: %s)"
			% [_cached_opponent_roster.size(), _opponent_team_name]
		)
	)


func _sort_roster_by_formation(roster: Array) -> Array:
	"""Sort roster by position to match 4-4-2 formation exactly:
	GK, LB, CB, CB, RB, LM, CM, CM, RM, ST, ST
	"""
	var sorted_roster: Array = []
	var by_position: Dictionary = {"GK": [], "LB": [], "CB": [], "RB": [], "LM": [], "CM": [], "RM": [], "ST": []}  # Defenders  # Midfielders  # Forwards

	# Categorize players by specific position
	for player in roster:
		var pos: String = player.get("position", "CM").to_upper()

		# Goalkeeper
		if pos == "GK":
			by_position["GK"].append(player)
		# Defenders
		elif pos == "LB" or pos == "LWB":
			by_position["LB"].append(player)
		elif pos == "CB" or pos == "DF":
			by_position["CB"].append(player)
		elif pos == "RB" or pos == "RWB":
			by_position["RB"].append(player)
		elif pos == "FB":
			# Full-back can play either side - add to LB (will be used as fallback for RB too)
			by_position["LB"].append(player)
		# Midfielders
		elif pos == "LM" or pos == "LW":
			by_position["LM"].append(player)
		elif pos == "CM" or pos == "CDM" or pos == "CAM" or pos == "MF" or pos == "DM" or pos == "AM":
			by_position["CM"].append(player)
		elif pos == "RM" or pos == "RW":
			by_position["RM"].append(player)
		# Forwards
		elif pos == "ST" or pos == "CF" or pos == "FW":
			by_position["ST"].append(player)
		else:
			# Unknown position, treat as CM
			by_position["CM"].append(player)

	# Build starting 11 in exact order: GK, LB, CB, CB, RB, LM, CM, CM, RM, ST, ST
	var formation_template := ["GK", "LB", "CB", "CB", "RB", "LM", "CM", "CM", "RM", "ST", "ST"]  # 1. Goalkeeper  # 2. Left Back  # 3. Center Back (left)  # 4. Center Back (right)  # 5. Right Back  # 6. Left Midfielder  # 7. Center Midfielder (left)  # 8. Center Midfielder (right)  # 9. Right Midfielder  # 10. Striker (left)  # 11. Striker (right)

	var used_indices: Dictionary = {}  # Track used player indices per position

	for template_pos in formation_template:
		var available_players: Array = by_position[template_pos]
		var next_idx: int = used_indices.get(template_pos, 0)

		if next_idx < available_players.size():
			sorted_roster.append(available_players[next_idx])
			used_indices[template_pos] = next_idx + 1
		else:
			# Not enough players for this position, use position-appropriate fallback
			var found: bool = false
			var fallback_order: Array = []

			# Position-specific fallback priority
			match template_pos:
				"ST":
					fallback_order = ["CM", "LM", "RM"]  # Attacking positions
				"LM", "RM":
					fallback_order = ["CM", "ST"]
				"CM":
					fallback_order = ["LM", "RM", "ST"]
				"LB":
					fallback_order = ["CB", "RB"]
				"RB":
					fallback_order = ["CB", "LB"]
				"CB":
					fallback_order = ["LB", "RB"]
				_:
					fallback_order = ["CM", "CB", "ST", "LM", "RM", "RB", "LB"]

			for fallback_pos in fallback_order:
				var fallback_idx: int = used_indices.get(fallback_pos, 0)
				if fallback_idx < by_position[fallback_pos].size():
					var fallback_player: Dictionary = by_position[fallback_pos][fallback_idx]
					# Force update position to match formation slot
					fallback_player["position"] = template_pos
					sorted_roster.append(fallback_player)
					used_indices[fallback_pos] = fallback_idx + 1
					print(
						"[MatchSimulationManager] ⚠️ Position shortage: Using %s as %s" % [fallback_pos, template_pos]
					)
					found = true
					break

			if not found:
				# Last resort: create emergency placeholder for this position
				var emergency_player: Dictionary = _generate_placeholder_player(sorted_roster.size(), template_pos)
				sorted_roster.append(emergency_player)
				print("[MatchSimulationManager] ⚠️ Emergency placeholder created for %s" % template_pos)

	# Add remaining players as substitutes
	for pos_key in ["GK", "LB", "CB", "RB", "LM", "CM", "RM", "ST"]:
		var start_idx: int = used_indices.get(pos_key, 0)
		for i in range(start_idx, by_position[pos_key].size()):
			if by_position[pos_key][i] not in sorted_roster:
				sorted_roster.append(by_position[pos_key][i])

	# Add any stragglers not yet added
	for player in roster:
		if player not in sorted_roster:
			sorted_roster.append(player)

	print(
		(
			"[MatchSimulationManager] 📋 Sorted roster for 4-4-2: GK=%d LB=%d CB=%d RB=%d LM=%d CM=%d RM=%d ST=%d"
			% [
				by_position["GK"].size(),
				by_position["LB"].size(),
				by_position["CB"].size(),
				by_position["RB"].size(),
				by_position["LM"].size(),
				by_position["CM"].size(),
				by_position["RM"].size(),
				by_position["ST"].size()
			]
		)
	)

	return sorted_roster


func _generate_placeholder_player(slot_index: int, position: String) -> Dictionary:
	var player_id: String = "placeholder_%d" % slot_index
	var ca: int = 48 + (slot_index % 7)
	var pa: int = min(ca + 12, 160)
	var canonical_position: String = _canonicalize_position(position, slot_index)

	var player_data: Dictionary = {}
	player_data["name"] = "Youth Player %02d" % slot_index
	player_data["player_id"] = player_id
	player_data["uid"] = 900000 + slot_index
	player_data["position"] = canonical_position
	player_data["position_category"] = _infer_position_category(canonical_position)
	player_data["age"] = 18
	player_data["technical"] = {}
	player_data["mental"] = {}
	player_data["physical"] = {}
	player_data["goalkeeper"] = {}
	player_data["ca"] = ca
	player_data["pa"] = pa
	player_data["overall"] = ca
	player_data["condition"] = 0.85
	player_data["form"] = 50
	player_data["morale"] = 50
	player_data["stamina"] = 100
	player_data["fatigue"] = 0.0
	player_data["personality"] = {}
	player_data["personality_archetype"] = "Steady"  # Default for new players

	# Parts 기반 랜덤 외형 생성
	player_data["appearance"] = _PlayerAppearanceBridge.create_random()

	_ensure_player_attribute_map(player_data)
	return player_data


func _normalize_match_condition(value) -> float:
	var cond: float = 0.8
	if typeof(value) == TYPE_INT or typeof(value) == TYPE_FLOAT:
		cond = float(value)
		if cond > 1.0:
			cond = clamp((cond - 1.0) / 4.0, 0.0, 1.0)
		else:
			cond = clamp(cond, 0.0, 1.0)
	return cond


func _infer_position_category(position: String) -> String:
	match position:
		"GK":
			return "골키퍼"
		"CB", "RB", "LB", "RWB", "LWB", "SW", "DF":
			return "수비수"
		"CM", "CDM", "CAM", "LM", "RM", "AM", "DM", "MF":
			return "미드필더"
		_:
			return "공격수"


func _find_saved_player_by_id(my_team: Node, player_id: String) -> Dictionary:
	if player_id == "":
		return {}
	for saved_player in my_team.saved_players:
		if saved_player.get("id", "") == player_id:
			return saved_player
	return {}


func _load_opponent_roster_from_cache(team_name: String) -> void:
	# DEPRECATED: OpponentRosterCache autoload doesn't exist, this function never worked
	var cache = get_node_or_null("/root/OpponentRosterCache")
	if cache == null:
		return

	if not cache.has_method("get_team") or not cache.has_method("get_random_team"):
		return

	var use_real_names := _use_real_names_for_opponents()
	var team_to_use := team_name
	var rows: Array = cache.get_team(team_name)
	if rows.is_empty():
		var fallback_team: String = cache.get_random_team(int(Time.get_ticks_msec()))
		if fallback_team != "":
			print(
				"[MatchSimulationManager] ⚠️ Cache missing team '%s', using fallback '%s'" % [team_name, fallback_team]
			)
			rows = cache.get_team(fallback_team)
			if not rows.is_empty():
				team_to_use = fallback_team
	if rows.is_empty():
		return

	_cached_opponent_roster.clear()
	_opponent_team_name = team_to_use
	if use_real_names and not rows.is_empty():
		var first_variant: Variant = rows[0]
		if first_variant is Dictionary:
			var first_dict: Dictionary = first_variant
			var real_team: String = String(first_dict.get("Team", _opponent_team_name)).strip_edges()
			if real_team != "":
				_opponent_team_name = real_team

	for slot_index in range(min(rows.size(), 18)):
		var csv_row_variant: Variant = rows[slot_index]
		if csv_row_variant is Dictionary:
			var csv_row: Dictionary = csv_row_variant
			var player := _create_opponent_player_from_row(csv_row, slot_index, use_real_names)
			_cached_opponent_roster.append(player)

	print(
		"[MatchSimulationManager] ✅ Loaded %d players from cache for '%s'" % [_cached_opponent_roster.size(), team_name]
	)


func _generate_opponent_players(opponent_ca: int, match_data: Dictionary) -> Array:
	"""Generate opponent roster using OpponentRosterProvider (Phase B)"""
	print("[MatchSimulationManager] 🔍 DEBUG: _generate_opponent_players() called (using OpponentRosterProvider)")

	var opponent_name: String = String(match_data.get("opponent", "Opponent Team"))
	var use_real_names: bool = _use_real_names_for_opponents()

	# ========================================================================
	# Phase B: Delegate to OpponentRosterProvider (stateless, no instance cache)
	# ========================================================================
	var template_roster: Array = OpponentRosterProvider.get_opponent_roster(opponent_ca, 18, use_real_names)
	print("[MatchSimulationManager] 📦 Received %d players from OpponentRosterProvider" % template_roster.size())

	var roster: Array = []

	if template_roster.size() >= 11:
		print("[MatchSimulationManager] ✅ Building roster from provider data (%d players)" % template_roster.size())
		for i in range(min(18, template_roster.size())):
			var template: Dictionary = template_roster[i]
			var player: Dictionary = template.duplicate(true)

			# Adjust CA based on opponent_ca requirement
			var desired_ca: int = clampi(opponent_ca + int(randf_range(-2.0, 2.0)), 45, 130)
			player["ca"] = desired_ca
			player["overall"] = desired_ca
			if int(player.get("pa", desired_ca + 10)) < desired_ca:
				player["pa"] = clampi(desired_ca + 10, desired_ca + 5, 199)

			# Reset match state
			player["condition"] = clamp(player.get("condition", 0.8) + randf_range(-0.05, 0.05), 0.6, 0.95)
			player["stamina"] = 100
			player["fatigue"] = 0.0
			player["form"] = 50
			player["morale"] = 50

			# Assign unique identifiers
			var player_id: String = "%s_%02d" % [opponent_name.to_lower().replace(" ", "_"), i + 1]
			player["player_id"] = player_id
			player["uid"] = abs(hash(player_id)) % 1000000000

			if String(player.get("name", "")) == "":
				player["name"] = "%s %02d" % [opponent_name, i + 1]

			_ensure_player_attribute_map(player)
			roster.append(player)

	if roster.size() < 11:
		push_warning("[MatchSimulationManager] ⚠️ Roster has < 11 players, falling back to dummy generation")
		roster = _build_fallback_opponent_roster(opponent_name, opponent_ca)
		print("[MatchSimulationManager] ⚠️ Using DUMMY players (generated %d players)" % roster.size())
	else:
		print("[MatchSimulationManager] ✅ Returning %d players" % roster.size())

	return roster


func _get_stage_manager_ref() -> Node:
	return get_node_or_null("/root/StageManager")


func _get_player_team_manager_payload() -> Dictionary:
	var stage_manager_ref = _get_stage_manager_ref()
	if stage_manager_ref and stage_manager_ref.has_method("get_player_manager_data"):
		var payload: Dictionary = stage_manager_ref.get_player_manager_data()
		if payload is Dictionary and not payload.is_empty():
			return payload.duplicate(true)
	return {}


func _get_stage_opponent_payload() -> Dictionary:
	if OpponentRosterProvider and OpponentRosterProvider.has_method("get_last_stage_team_payload"):
		var payload: Dictionary = OpponentRosterProvider.get_last_stage_team_payload()
		if payload is Dictionary and not payload.is_empty():
			return payload.duplicate(true)
	return {}


func _get_stage_opponent_manager_payload() -> Dictionary:
	if OpponentRosterProvider and OpponentRosterProvider.has_method("get_last_stage_manager_data"):
		var payload: Dictionary = OpponentRosterProvider.get_last_stage_manager_data()
		if payload is Dictionary and not payload.is_empty():
			return payload.duplicate(true)
	return {}


func _build_opponent_roster_from_player_cache(opponent_ca: int, match_data: Dictionary) -> bool:
	# DEPRECATED: Use OpponentRosterProvider.get_opponent_roster() instead
	var game_cache = get_node_or_null("/root/GameCache")
	if game_cache == null:
		return false
	if not game_cache.has_method("is_player_cache_ready") or not game_cache.is_player_cache_ready():
		return false
	if not game_cache.has_method("get_balanced_roster"):
		push_warning("[MatchSimulationManager] Player cache ready but balanced roster API missing")
		return false

	var opponent_name: String = String(match_data.get("opponent", "Opponent Team"))
	var use_real_names := _use_real_names_for_opponents()
	var min_ca: int = clampi(opponent_ca - 8, 35, 180)
	var roster_entries: Array = game_cache.get_balanced_roster(min_ca, 18, use_real_names)
	if roster_entries.is_empty():
		min_ca = clampi(opponent_ca - 15, 30, 170)
		roster_entries = game_cache.get_balanced_roster(min_ca, 18, use_real_names)
	if roster_entries.is_empty():
		return false

	_cached_opponent_roster.clear()
	_opponent_team_name = opponent_name

	for slot_index in range(min(roster_entries.size(), 18)):
		var entry_variant: Variant = roster_entries[slot_index]
		if entry_variant is Dictionary:
			var entry: Dictionary = entry_variant
			var player := _create_opponent_player_from_cache_entry(
				entry, slot_index, opponent_name, opponent_ca, use_real_names
			)
			_cached_opponent_roster.append(player)

	return _cached_opponent_roster.size() >= 11


func _build_fallback_opponent_roster(team_name: String, opponent_ca: int) -> Array:
	var roster: Array = []
	var base_name: String = team_name.strip_edges()
	if base_name == "":
		base_name = "Opponent"

	for i in range(min(MATCH_POSITION_TEMPLATE.size(), 18)):
		var slot_index: int = i
		var position: String = MATCH_POSITION_TEMPLATE[min(i, MATCH_POSITION_TEMPLATE.size() - 1)]
		var player: Dictionary = _generate_placeholder_player(slot_index, position)
		var display_index: int = slot_index + 1
		player["name"] = "%s %02d" % [base_name, display_index]
		var player_id: String = "%s_%02d" % [base_name.to_lower().replace(" ", "_"), display_index]
		player["player_id"] = player_id
		player["uid"] = abs(hash(player_id)) % 1000000000
		var desired_ca: int = clampi(opponent_ca + int(randf_range(-3.0, 3.0)), 45, 120)
		player["ca"] = desired_ca
		player["overall"] = desired_ca
		player["pa"] = clampi(desired_ca + 10, desired_ca + 5, 199)
		player["condition"] = clamp(player.get("condition", 0.8) + randf_range(-0.05, 0.05), 0.65, 0.95)
		_ensure_player_attribute_map(player)
		roster.append(player)

	while roster.size() < 18:
		var slot_index_fallback: int = roster.size()
		var fallback_position: String = MATCH_POSITION_TEMPLATE[min(
			slot_index_fallback, MATCH_POSITION_TEMPLATE.size() - 1
		)]
		var extra_player: Dictionary = _generate_placeholder_player(slot_index_fallback, fallback_position)
		extra_player["name"] = "%s %02d" % [base_name, slot_index_fallback + 1]
		var extra_id: String = "%s_%02d" % [base_name.to_lower().replace(" ", "_"), slot_index_fallback + 1]
		extra_player["player_id"] = extra_id
		extra_player["uid"] = abs(hash(extra_id)) % 1000000000
		extra_player["ca"] = clampi(opponent_ca + int(randf_range(-4.0, 4.0)), 40, 115)
		extra_player["overall"] = extra_player["ca"]
		extra_player["pa"] = clampi(extra_player["ca"] + 10, extra_player["ca"] + 5, 199)
		_ensure_player_attribute_map(extra_player)
		roster.append(extra_player)

	return roster


# ============================================================================
# DEBUG & TESTING
# ============================================================================


func debug_trigger_match(week: int = 12, year: int = 1) -> void:
	"""Debug: Manually trigger a match"""
	var match_data = _get_match_for_week(week, year)

	if match_data.is_empty():
		push_warning("[MatchSimulationManager] No match found for Week %d, Year %d" % [week, year])
		return

	print("[MatchSimulationManager] 🐛 DEBUG: Triggering match at Week %d, Year %d" % [week, year])
	_prepare_match(match_data)
	await simulate_match(match_data)


func get_match_history() -> Array:
	"""Get all completed matches"""
	return match_history


func get_upcoming_matches() -> Array:
	"""Get upcoming matches for current year"""
	return upcoming_matches


func get_match_statistics() -> Dictionary:
	"""Get match statistics summary"""
	var wins = 0
	var draws = 0
	var losses = 0
	var total_goals = 0
	var total_assists = 0

	for entry in match_history:
		var home = entry.get("home_score", 0)
		var away = entry.get("away_score", 0)

		if home > away:
			wins += 1
		elif home == away:
			draws += 1
		else:
			losses += 1

		total_goals += entry.get("goals", 0)
		total_assists += entry.get("assists", 0)

	return {
		"matches_played": match_history.size(),
		"wins": wins,
		"draws": draws,
		"losses": losses,
		"total_goals": total_goals,
		"total_assists": total_assists
	}


func queue_substitution(payload: Dictionary) -> void:
	"""Queue a substitution request during a match session and forward to Rust engine."""
	if payload.is_empty():
		push_warning("[MatchSimulationManager] Received empty substitution payload")
		return

	var normalized_team := _normalize_team_label(payload.get("team", "home"))
	var team_id := _team_label_to_id(normalized_team)

	# SSOT payload (preferred):
	# - out_track_id: pitch slot (0..21)
	# - in_bench_slot: per-team bench slot (0..6)
	var out_track_id := -1
	if payload.has("out_track_id"):
		out_track_id = int(payload.get("out_track_id", -1))
	else:
		# Back-compat: allow roster_id formats ("H0", "A3") for OUT only
		var out_roster_id := str(payload.get("out_player_id", payload.get("out", "")))
		out_track_id = _roster_id_to_track_id(out_roster_id, team_id)

	var in_bench_slot := -1
	if payload.has("in_bench_slot"):
		in_bench_slot = int(payload.get("in_bench_slot", -1))
	elif payload.has("bench_slot"):
		in_bench_slot = int(payload.get("bench_slot", -1))
	else:
		# Back-compat: allow numeric string (0..6)
		var in_raw := str(payload.get("in_player_id", payload.get("in", "")))
		if in_raw.is_valid_int():
			in_bench_slot = int(in_raw)

	var out_name := str(payload.get("out_name", payload.get("out_player_id", payload.get("out", ""))))
	var in_name := str(payload.get("in_name", payload.get("in_player_id", payload.get("in", ""))))

	var sanitized := {
		"minute": float(payload.get("minute", 0)),
		"team": normalized_team,
		"out_track_id": out_track_id,
		"in_bench_slot": in_bench_slot,
		"out_name": out_name,
		"in_name": in_name,
	}

	if sanitized.out_track_id < 0 or sanitized.out_track_id > 21:
		push_warning("[MatchSimulationManager] Substitution ignored: invalid out_track_id=%s" % str(sanitized.out_track_id))
		return
	if sanitized.in_bench_slot < 0 or sanitized.in_bench_slot > 6:
		push_warning("[MatchSimulationManager] Substitution ignored: invalid in_bench_slot=%s" % str(sanitized.in_bench_slot))
		return

	# Team sanity (pitch-slot split: home 0..10, away 11..21)
	if sanitized.team == "home" and sanitized.out_track_id > 10:
		push_warning("[MatchSimulationManager] Substitution ignored: home out_track_id must be 0..10 (got %d)" % int(sanitized.out_track_id))
		return
	if sanitized.team == "away" and sanitized.out_track_id < 11:
		push_warning("[MatchSimulationManager] Substitution ignored: away out_track_id must be 11..21 (got %d)" % int(sanitized.out_track_id))
		return

	var rust_engine := _get_rust_match_engine()
	var response: Dictionary = {}
	if rust_engine and rust_engine.has_method("apply_session_substitution_from_payload"):
		response = rust_engine.apply_session_substitution_from_payload(sanitized)
		if typeof(response) == TYPE_DICTIONARY and not bool(response.get("success", true)):
			var error_msg := str(response.get("error", ""))
			if error_msg != "":
				push_warning("[MatchSimulationManager] Rust substitution bridge warning: %s" % error_msg)
	else:
		push_warning("[MatchSimulationManager] Rust engine unavailable; recording substitution locally")

	var event: Dictionary = {}
	if typeof(response) == TYPE_DICTIONARY and response.has("event"):
		event = response.event

	if not (event is Dictionary) or event.is_empty():
		event = _build_substitution_event(sanitized)
	else:
		if not event.has("team"):
			event["team"] = sanitized.team
		if not event.has("player_out"):
			event["player_out"] = sanitized.out_name
		if not event.has("player_in"):
			event["player_in"] = sanitized.in_name
		if not event.has("minute"):
			event["minute"] = sanitized.minute
			var base_variant: Variant = event.get("base", {})
			var base_dict: Dictionary = base_variant if base_variant is Dictionary else {}
			if not base_dict.has("t"):
				base_dict["t"] = sanitized.minute
			if not base_dict.has("team_id"):
				base_dict["team_id"] = _team_label_to_id(sanitized.team)
			if not base_dict.has("player_id"):
				base_dict["player_id"] = sanitized.out_name
			if not base_dict.has("player_track_id"):
				base_dict["player_track_id"] = sanitized.out_track_id
			event["base"] = base_dict

	print(
		(
			"[MatchSimulationManager] Substitution queued: %s → %s (minute %.0f)"
			% [sanitized.out_name, sanitized.in_name, sanitized.minute]
		)
	)

	_append_session_event(event)


func apply_tactics(payload: Dictionary) -> void:
	"""Apply tactical changes during a match session and forward to Rust tactical engine."""
	if payload.is_empty():
		push_warning("[MatchSimulationManager] Empty tactical payload ignored")
		return

	var formation := str(payload.get("formation", player_formation))
	var attack_bias := clampf(float(payload.get("attack_bias", 0.5)), 0.0, 1.0)
	var press_intensity := clampf(float(payload.get("press_intensity", 0.5)), 0.0, 1.0)
	var tempo := str(payload.get("tempo", "normal"))
	var minute := float(payload.get("minute", 0))
	var team_label := _normalize_team_label(payload.get("team", "home"))

	# Forward the full payload (team/preset/instructions) so TacticalEngine can coerce to TeamInstructions.
	var engine_payload := payload.duplicate(true)
	engine_payload["team"] = team_label
	engine_payload["formation"] = formation
	engine_payload["attack_bias"] = attack_bias
	engine_payload["press_intensity"] = press_intensity
	engine_payload["tempo"] = tempo
	engine_payload["minute"] = minute

	var response := _forward_tactics_to_engine(engine_payload)
	if typeof(response) == TYPE_DICTIONARY and not bool(response.get("success", true)):
		var error_msg := str(response.get("error", ""))
		if error_msg != "":
			push_warning("[MatchSimulationManager] Tactical update warning: %s" % error_msg)

	player_formation = formation
	player_instructions["attack_bias"] = attack_bias * 100.0
	player_instructions["press_intensity"] = press_intensity * 100.0
	player_instructions["tempo"] = tempo

	print(
		(
			"[MatchSimulationManager] Tactical update applied → Formation: %s, Attack: %.0f, Press: %.0f, Tempo: %s"
			% [
				player_formation,
				player_instructions["attack_bias"],
				player_instructions["press_intensity"],
				player_instructions["tempo"]
			]
		)
	)

	var timeline_payload := {
		"formation": formation,
		"attack_bias": attack_bias,
		"press_intensity": press_intensity,
		"tempo": tempo,
		"minute": minute,
		"team": team_label
	}
	_append_session_event(_build_tactical_event(timeline_payload))


func _handle_halftime_update() -> void:
	if not MatchControlState:
		return

	var latest_tactics := MatchControlState.get_current_tactics()
	if latest_tactics is Dictionary and not latest_tactics.is_empty():
		var engine_payload := {
			"formation": str(latest_tactics.get("formation", player_formation)),
			"attack_bias": clampf(float(latest_tactics.get("attack_bias", 0.5)), 0.0, 1.0),
			"press_intensity": clampf(float(latest_tactics.get("press_intensity", 0.5)), 0.0, 1.0),
			"tempo": str(latest_tactics.get("tempo", "normal"))
		}
		var response := _forward_tactics_to_engine(engine_payload)
		if typeof(response) == TYPE_DICTIONARY and not bool(response.get("success", true)):
			var error_msg := str(response.get("error", ""))
			if error_msg != "":
				push_warning("[MatchSimulationManager] Half-time tactics reapply warning: %s" % error_msg)

	var substitution_log := MatchControlState.get_substitution_log()
	if substitution_log is Array and substitution_log.size() > 0:
		print("[MatchSimulationManager] Half-time substitution state: %d changes recorded" % substitution_log.size())


# =============================================================================
# 플레이어 2D 렌더링 헬퍼 (DEPRECATED - 2025-12-09)
# =============================================================================
# NOTE: Legacy SkeletonPlayer/Skeleton2DPlayer system has been removed.
# Match viewer now uses HorizontalMatchViewer with Socceralia sprites (SoccerPlayer).
# See: docs/spec+@/spec_v4/dev_spec/UI/2025-12-08_CHARACTER_SPRITE_INTEGRATION_SPEC.md

# =============================================================================
# Session callback bridge (deprecated)
# =============================================================================
# Snapshot emission is handled by UnifiedFramePipeline.
# This placeholder remains for future callback-based wiring, but is currently unused.

# =============================================================================
# Career Player Mode: User Command System (ST-005 Phase 3 - Delegated)
# =============================================================================
# Implementation moved to UserCommandDispatcher.gd
# These are thin wrappers for backward compatibility

func send_user_command(cmd: Dictionary) -> void:
	if _user_command_dispatcher:
		_user_command_dispatcher.send_user_command(cmd)

func register_controller_slot(controller_id: int, team_side: String, player_slot: int) -> void:
	if _user_command_dispatcher:
		_user_command_dispatcher.register_controller_slot(controller_id, team_side, player_slot)

func unregister_controller_slot(controller_id: int) -> void:
	if _user_command_dispatcher:
		_user_command_dispatcher.unregister_controller_slot(controller_id)

func clear_controller_slots() -> void:
	if _user_command_dispatcher:
		_user_command_dispatcher.clear_controller_slots()

func send_multi_agent_commands(commands: Array) -> void:
	if _user_command_dispatcher:
		_user_command_dispatcher.send_multi_agent_commands(commands)

func set_sticky_action(track_id: int, action: String, enabled: bool) -> void:
	if _user_command_dispatcher:
		_user_command_dispatcher.set_sticky_action(track_id, action, enabled)


## Preflight validation: Check if all roster UIDs can be resolved
func _preflight_validate_roster(match_setup: _MatchSetup) -> Dictionary:
	if match_setup == null:
		return {"ok": false, "error": "MatchSetup is null"}
	if match_setup.home_team == null or match_setup.away_team == null:
		return {"ok": false, "error": "Team objects missing"}

	var missing: Array = []

	# Extract home team starter UIDs
	for p in match_setup.home_team.starters:
		var uid := ""
		if p is _MatchPlayer:
			uid = str(p.uid)
		else:
			# Fallback for Dictionary or other types - use Variant to avoid type narrowing
			var item: Variant = p
			if item is Dictionary and item.has("uid"):
				uid = str(item["uid"])
			elif item != null and item.has_method("get_uid"):
				uid = str(item.call("get_uid"))

		if uid != "" and not _can_resolve_uid(uid):
			missing.append("home:" + uid)

	# Extract away team starter UIDs
	for p in match_setup.away_team.starters:
		var uid := ""
		if p is _MatchPlayer:
			uid = str(p.uid)
		else:
			# Fallback for Dictionary or other types - use Variant to avoid type narrowing
			var item: Variant = p
			if item is Dictionary and item.has("uid"):
				uid = str(item["uid"])
			elif item != null and item.has_method("get_uid"):
				uid = str(item.call("get_uid"))

		if uid != "" and not _can_resolve_uid(uid):
			missing.append("away:" + uid)

	if missing.size() > 0:
		return {
			"ok": false, "error": "Roster validation failed - missing players: %s" % str(missing), "missing": missing
		}

	return {"ok": true, "error": ""}


## Check if a UID can be resolved (cache-first strategy)
func _can_resolve_uid(uid: String) -> bool:
	# 1) Engine cache (priority)
	if has_node("/root/FootballRustEngine"):
		var engine = get_node("/root/FootballRustEngine")
		if engine and engine.has_method("has_player_uid"):
			if bool(engine.call("has_player_uid", uid)):
				return true

			# Try numeric variant (csv:2 -> 2)
			if uid.begins_with("csv:"):
				var n := uid.substr(4)
				if n.is_valid_int():
					if bool(engine.call("has_player_uid", n)):
						return true

	# 2) PlayerLibrary fallback (needs "csv:" prefix for numeric UIDs)
	if _player_library != null and _player_library.has_method("get_player_data"):
		var lookup_uid := uid
		# Convert numeric UID to csv: format for PlayerLibrary
		if uid.is_valid_int():
			lookup_uid = "csv:%s" % uid
		var d = _player_library.get_player_data(lookup_uid)
		if d is Dictionary and not d.is_empty():
			return true

	return false


# UID Resolution helpers (ST-005 Phase 2 - Delegated to RosterBuilder)
func _resolve_engine_uid_for_main_player(raw_uid: Variant) -> String:
	if _roster_builder:
		return _roster_builder.resolve_engine_uid_for_main_player(raw_uid)
	return ""


func _pick_valid_engine_uid_near(center: int) -> String:
	if _roster_builder:
		return _roster_builder.pick_valid_engine_uid_near(center)
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
