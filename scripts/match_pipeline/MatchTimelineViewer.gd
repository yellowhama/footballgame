extends Control
class_name MatchTimelineViewer
# Preload to avoid autoload order issues with class_name
const _TimelinePlayerMapper = preload("res://scripts/match/TimelinePlayerMapper.gd")
##
## MatchTimelineViewer - Match Scene Viewer
##
## This is NOT just a Timeline viewer - it's designed for REAL-TIME match observation
## where users can watch the match and intervene (play cards, change tactics).
##
## Main Use Case (80%): Session Match View
##   - Camera follows ball (ViewMode.CAMERA_FOLLOW)
##   - Minimap overlay shows full formation (top-right corner)
##   - Clean UI with essential info only (shots, cards, hero player)
##   - Optimized for mobile portrait screens (1080x1920)
##
## Secondary Use Case (20%): Tactical Analysis
##   - Full landscape view (ViewMode.FULL_LANDSCAPE)
##   - All overlays enabled (pass lines, pressure, runs, heat map)
##
## Quick Start:
##   var viewer = MatchTimelineViewer.new()
##   viewer.apply_preset_session_match()  # or apply_preset_tactical_analysis()
##   viewer.load_timeline_data(events, rosters, metadata)
##   viewer.play()
##
## See: docs/spec+@/spec_v4/dev_spec/view/MatchSceneViewer_Usage_Guide.md
##
## Legacy Note: 기존 MiniMap + 2D Simple viewer 통합
##

# Note: TimelinePlayerMapper has class_name, so it's globally available

#region Constants
const FIELD_LENGTH: float = 105.0
const FIELD_WIDTH: float = 68.0
const FIELD_PADDING: float = 8.0
const PENALTY_LENGTH: float = 16.5
const PENALTY_WIDTH: float = 40.3

const HOME_COLOR: Color = Color(0.85, 0.2, 0.2, 1.0)
const AWAY_COLOR: Color = Color(0.2, 0.4, 0.85, 1.0)
const FIELD_COLOR: Color = Color(0.22, 0.55, 0.22, 1.0)
const LINE_COLOR: Color = Color(1.0, 1.0, 1.0, 0.7)
const BORDER_COLOR: Color = Color(0.15, 0.15, 0.15, 1.0)
const BALL_COLOR: Color = Color(1.0, 1.0, 1.0, 1.0)

const PLAYER_SIZE: Vector2 = Vector2(12.0, 12.0)
const BALL_RADIUS: float = 6.0
const HIGHLIGHT_DURATION: float = 1.2
const POSITION_SMOOTH_SPEED: float = 12.0
const BALL_SMOOTH_SPEED: float = 18.0
const SMOOTH_EPSILON: float = 0.05

const BALL_TRAIL_MAX_POINTS: int = 20
const BALL_TRAIL_FADE_SPEED: float = 3.0
const PLAYER_TRAIL_MAX_POINTS: int = 15
const BALL_BOUNCE_THRESHOLD: float = 0.3

const MAX_OVERLAY_ENTRIES: int = 50
const MAX_HEAT_SAMPLES: int = 100
const GOAL_HEAT_REFERENCE_XG: float = 2.5

const PASS_COLOR: Color = Color(0.6, 0.9, 0.6, 0.7)
const SHOT_COLOR: Color = Color(1.0, 0.85, 0.2, 0.85)
const DRIBBLE_COLOR: Color = Color(0.95, 0.65, 0.1, 0.6)
const THROUGH_BALL_COLOR: Color = Color(0.3, 0.95, 0.95, 0.75)
const RUN_COLOR: Color = Color(0.5, 0.7, 1.0, 0.5)
const RUN_WITH_BALL_COLOR: Color = Color(1.0, 0.6, 0.2, 0.65)

const POSITION_NORMALS: Dictionary = {
	"GK": Vector2(0.05, 0.5),
	"LB": Vector2(0.15, 0.17),
	"RB": Vector2(0.15, 0.83),
	"LCB": Vector2(0.15, 0.35),
	"RCB": Vector2(0.15, 0.65),
	"CB": Vector2(0.15, 0.5),
	"LWB": Vector2(0.22, 0.12),
	"RWB": Vector2(0.22, 0.88),
	"LDM": Vector2(0.28, 0.35),
	"RDM": Vector2(0.28, 0.65),
	"DM": Vector2(0.28, 0.5),
	"LCM": Vector2(0.4, 0.35),
	"RCM": Vector2(0.4, 0.65),
	"CM": Vector2(0.4, 0.5),
	"LM": Vector2(0.45, 0.15),
	"RM": Vector2(0.45, 0.85),
	"LAM": Vector2(0.55, 0.32),
	"RAM": Vector2(0.55, 0.68),
	"AM": Vector2(0.55, 0.5),
	"LW": Vector2(0.7, 0.12),
	"RW": Vector2(0.7, 0.88),
	"SS": Vector2(0.72, 0.5),
	"CF": Vector2(0.8, 0.5),
	"ST": Vector2(0.85, 0.5),
}
#endregion

#region Enums
enum ViewMode { FULL_PORTRAIT, FULL_LANDSCAPE, CAMERA_FOLLOW }  ## 세로 전체화면 (미니맵용)  ## 가로 전체화면 (경기장 배경용)  ## 가로 뷰를 세로 카메라로 촬영 (볼 추적)
#endregion

#region Export Variables
@export_group("Basic")
@export var view_mode: ViewMode = ViewMode.FULL_PORTRAIT
@export var portrait_mode: bool = true  ## Deprecated: use view_mode instead
@export var auto_play: bool = true
@export var default_speed: float = 1.0

@export_group("Camera Follow")
@export var camera_safe_margin: Vector2 = Vector2(150.0, 100.0)  ## 볼 주변 여유 공간 (픽셀)
@export var camera_smooth_speed: float = 5.0  ## 카메라 이동 속도
@export var camera_zoom: float = 1.8  ## 카메라 확대 비율
@export var camera_viewport_size: Vector2 = Vector2(1080, 1920)  ## 출력 뷰포트 크기

@export_group("Ball Animation")
@export var enable_ball_height: bool = true
@export var enable_ball_squash_stretch: bool = true
@export var enable_ball_trail: bool = true
@export var enable_ball_rotation: bool = true
@export var enable_ball_bounce: bool = true

@export_group("Player Animation")
@export var enable_player_trails: bool = true
@export var enable_player_shadows: bool = true
@export var enable_player_stretch: bool = true
@export var enable_hero_highlight: bool = true
@export var show_stamina_bars: bool = true  ## P6: 선수 스태미나 바 표시
@export var stamina_bar_size: Vector2 = Vector2(16.0, 3.0)  ## 스태미나 바 크기

@export_group("Event Overlays")
@export var show_pass_lines: bool = true
@export var show_shot_points: bool = true
@export var show_pressure_points: bool = true
@export var show_run_segments: bool = true
@export var show_dribble_segments: bool = true
@export var show_throughball_lines: bool = true
@export var show_cards: bool = true
@export var show_assists: bool = true

@export_group("Heat Map")
@export var enable_heat_map: bool = true
@export var heat_map_opacity: float = 0.3

@export_group("Minimap Overlay")
@export var show_minimap: bool = true  ## CAMERA_FOLLOW 모드에서 미니맵 표시 여부
@export var minimap_position: Vector2 = Vector2(16, 16)  ## 미니맵 위치 (좌상단 기준)
@export var minimap_size: Vector2 = Vector2(180, 280)  ## 미니맵 크기
@export var minimap_opacity: float = 0.85  ## 미니맵 배경 투명도
@export var minimap_border_color: Color = Color(1.0, 1.0, 1.0, 0.6)  ## 미니맵 테두리 색상
@export var minimap_camera_rect_color: Color = Color(1.0, 0.9, 0.3, 0.7)  ## 현재 카메라 영역 표시 색상

@export_group("DSA Overlay")
@export var show_dsa_overlay: bool = true  ## FIX_2601/0114: DSA insight overlay (data-only)

@export_group("Position Playback")
@export var use_position_playback: bool = true  ## Deprecated: position_data-based playback (prefer UnifiedFramePipeline snapshots)
@export var auto_connect_controller: bool = true  ## Deprecated: viewer should not auto-connect to controllers
@export var sync_timeline_with_controller: bool = true  ## Deprecated: playhead sync is handled by timeline UI controls
#endregion

#region Signals
signal playback_started
signal playback_paused
signal playback_stopped
signal playback_finished
signal time_changed(current_time: float)
signal event_triggered(event: Dictionary)
signal player_highlighted(player_id: int)
#endregion

#region Internal State
var _events: Array = []
var _rosters: Dictionary = {}
var _metadata: Dictionary = {}
var _player_meta: Dictionary = {}

## Camera Follow State
var _camera_position: Vector2 = Vector2.ZERO  ## 현재 카메라 위치 (landscape 좌표계)
var _camera_target: Vector2 = Vector2.ZERO  ## 목표 카메라 위치

var _is_playing: bool = false
var _play_speed: float = 1.0
var _current_time: float = 0.0
var _max_time: float = 0.0
var _event_index: int = 0

var _ball_position: Vector2 = Vector2(FIELD_LENGTH * 0.5, FIELD_WIDTH * 0.5)
var _ball_display_position: Vector2 = Vector2(FIELD_LENGTH * 0.5, FIELD_WIDTH * 0.5)
var _ball_z: float = 0.0
var _ball_display_radius: float = BALL_RADIUS
var _ball_squash_scale: Vector2 = Vector2(1.0, 1.0)
var _ball_brightness: float = 1.0
var _ball_rotation: float = 0.0
var _ball_rotation_speed: float = 5.0
var _ball_trail_points: Array = []
var _ball_trail_active: bool = false
var _ball_squash_animating: bool = false
var _prev_ball_z: float = 0.0
var _current_height_profile: String = ""

var _player_positions: Dictionary = {}
var _display_positions: Dictionary = {}
var _player_trail_points: Dictionary = {}
var _highlight_timers: Dictionary = {}
var _assist_timers: Dictionary = {}
var _card_status: Dictionary = {}
var _player_stamina: Dictionary = {}  # P6: player_id -> stamina (0.0~1.0)

# FIX_2601/0114: DSA v1 (read-only telemetry; computed by DistributedSensingManager)
var _dsa_frame: Dictionary = {}

var _home_markers: Array = []
var _away_markers: Array = []
var _player_id_to_name: Dictionary = {}
var _player_identifier_to_marker: Dictionary = {}

var _home_engine_team_id: int = 1
var _away_engine_team_id: int = 2

var _pass_lines: Array = []
var _shot_points: Array = []
var _pressure_points: Array = []
var _run_segments: Array = []
var _dribble_segments: Array = []
var _throughball_lines: Array = []
var _communication_events: Array = []
var _header_events: Array = []

var _goal_heat_samples: Dictionary = {0: [], 1: []}
var _goal_heat_totals: Dictionary = {0: 0.0, 1: 0.0}
var _goal_heat_locked: bool = false

var _hero_player_id: String = ""
var _score_home: int = 0
var _score_away: int = 0
var _home_team_name: String = "Home"
var _away_team_name: String = "Away"

var _use_synthetic_timing: bool = false
var _snapshot_applied: bool = false
#endregion


#region Lifecycle
func _ready() -> void:
	set_process(false)
	_update_minimap_position_for_view()
	queue_redraw()

	# FIX_2601/0114: Subscribe to DSA insights (data-only; avoid render-loop compute)
	var dsa = get_node_or_null("/root/DistributedSensingManager")
	if dsa and dsa.has_signal("insight_frame_ready"):
		dsa.insight_frame_ready.connect(_on_dsa_insight_frame)
	# Match OS unification: this viewer must not subscribe to controller-provided snapshot signals.
	# Snapshots are consumed from UnifiedFramePipeline (directly or via a parent controller).


func _notification(what: int) -> void:
	if what == NOTIFICATION_RESIZED:
		_update_minimap_position_for_view()
		queue_redraw()


#endregion


#region Public API - Data Loading
func load_timeline_data(
	events: Array, rosters: Dictionary = {}, metadata: Dictionary = {}, stored_events: Array = []
) -> void:
	_events = events.duplicate(true) if events is Array else []

	_use_synthetic_timing = _should_use_synthetic_timing(_events)
	if _use_synthetic_timing and not _events.is_empty():
		var synthetic_interval: float = 0.5
		for i in range(_events.size()):
			if _events[i] is Dictionary:
				_events[i]["__synthetic_time"] = float(i) * synthetic_interval
		push_warning("[MatchTimelineViewer] Using synthetic timing (raw data unchanged).")
	else:
		_events.sort_custom(func(a, b): return _event_time(a) < _event_time(b))

	_rosters = rosters.duplicate(true) if rosters is Dictionary else {}
	_metadata = metadata.duplicate(true) if metadata is Dictionary else {}

	_player_meta.clear()
	if _TimelinePlayerMapper and _rosters is Dictionary:
		var mapper_doc := {
			"rosters": _rosters, "teams": _metadata.get("teams", {}), "match_id": _metadata.get("match_id", "")
		}
		_player_meta = _TimelinePlayerMapper.build_player_meta(mapper_doc, {})

	_process_stored_events(stored_events)
	_import_metadata()

	_max_time = 0.0
	for event in _events:
		_max_time = max(_max_time, _event_time(event))

	_update_engine_team_ids()
	_build_markers()
	restart()
	set_process(not _events.is_empty())
	queue_redraw()

	if auto_play:
		play()


func apply_position_snapshot(snapshot: Dictionary) -> void:
	if snapshot.is_empty():
		return
	var changed := false
	if snapshot.has("ball"):
		var ball_entry: Variant = snapshot.get("ball")
		var ball_vec := Vector2.ZERO
		if ball_entry is Dictionary and ball_entry.has("pos"):
			ball_vec = _vector_from_variant(ball_entry.get("pos"))
			# P6: Read ball height (z) from snapshot
			if enable_ball_height:
				var new_z: float = float(ball_entry.get("z", ball_entry.get("height", 0.0)))
				if abs(new_z - _ball_z) > 0.001:
					_prev_ball_z = _ball_z
					_ball_z = clamp(new_z, 0.0, 2.0)
		else:
			ball_vec = _vector_from_variant(ball_entry)
		_ball_position = _clamp_to_field(ball_vec)
		if not _snapshot_applied:
			_ball_display_position = _ball_position
		changed = true
	if snapshot.has("players") and snapshot.players is Dictionary:
		var players_dict: Dictionary = snapshot.players
		for key in players_dict.keys():
			var entry_variant: Variant = players_dict[key]
			if not (entry_variant is Dictionary):
				continue
			var entry: Dictionary = entry_variant
			var pos_variant: Variant = entry.get("pos", entry.get("position", null))
			if pos_variant == null:
				continue
			var marker_id := _resolve_marker_for_snapshot_key(key)
			if marker_id < 0:
				continue
			var pos_vec := _vector_from_variant(pos_variant)
			_player_positions[marker_id] = _clamp_to_field(pos_vec)
			if not _display_positions.has(marker_id):
				_display_positions[marker_id] = _player_positions[marker_id]
			# P6: Read stamina from snapshot
			if show_stamina_bars:
				var stamina: float = float(entry.get("stamina", 1.0))
				_player_stamina[marker_id] = clamp(stamina, 0.0, 1.0)
			changed = true
	_snapshot_applied = changed or _snapshot_applied
	if changed:
		queue_redraw()


#endregion


#region Public API - Playback Control
func play() -> void:
	if _events.is_empty():
		return
	_is_playing = true
	set_process(true)
	playback_started.emit()


func pause() -> void:
	_is_playing = false
	playback_paused.emit()


func stop() -> void:
	_is_playing = false
	_current_time = 0.0
	_event_index = 0
	playback_stopped.emit()
	queue_redraw()


func restart() -> void:
	_is_playing = false
	_current_time = 0.0
	_event_index = 0
	_ball_position = Vector2(FIELD_LENGTH * 0.5, FIELD_WIDTH * 0.5)
	_ball_display_position = _ball_position
	_display_positions.clear()
	_highlight_timers.clear()
	_assist_timers.clear()
	_card_status.clear()
	_ball_trail_points.clear()
	_player_trail_points.clear()
	_snapshot_applied = false
	queue_redraw()


func set_speed(multiplier: float) -> void:
	_play_speed = clamp(multiplier, 0.1, 4.0)


func jump_to(time_seconds: float) -> void:
	_current_time = clamp(time_seconds, 0.0, _max_time)
	_event_index = 0
	_ball_position = Vector2(FIELD_LENGTH * 0.5, FIELD_WIDTH * 0.5)
	_highlight_timers.clear()
	while _event_index < _events.size() and _event_time(_events[_event_index]) <= _current_time:
		_apply_event(_events[_event_index])
		_event_index += 1
	time_changed.emit(_current_time)
	queue_redraw()


func get_duration() -> float:
	return _max_time


func get_current_time() -> float:
	return _current_time


func is_playing() -> bool:
	return _is_playing


#endregion


#region Public API - Getters
func get_events() -> Array:
	return _events


func get_score() -> Dictionary:
	return {"home": _score_home, "away": _score_away}


func get_team_names() -> Dictionary:
	return {"home": _home_team_name, "away": _away_team_name}


func get_hero_player_id() -> String:
	return _hero_player_id


func set_hero_player_id(player_id: String) -> void:
	_hero_player_id = player_id
	var player_int_id := int(player_id) if player_id.is_valid_int() else 0
	if player_int_id != 0:
		player_highlighted.emit(player_int_id)
	queue_redraw()


#endregion


#region Public API - Presets
## Apply preset for Session match viewing (main use case - 80%)
## Optimized for real-time match observation with user intervention
func apply_preset_session_match() -> void:
	view_mode = ViewMode.CAMERA_FOLLOW

	# Ball animations
	enable_ball_trail = true
	enable_ball_height = true
	enable_ball_rotation = true
	enable_ball_squash_stretch = true
	enable_ball_bounce = true

	# Player animations
	enable_player_shadows = true
	enable_hero_highlight = true
	enable_player_trails = false  # Reduce screen complexity
	enable_player_stretch = false

	# Event overlays (essential only)
	show_shot_points = true
	show_cards = true
	show_assists = true
	show_pass_lines = false
	show_pressure_points = false
	show_run_segments = false
	show_dribble_segments = false
	show_throughball_lines = false
	enable_heat_map = false

	# Minimap overlay (top-right)
	show_minimap = true
	_update_minimap_position_for_view()

	queue_redraw()


## Apply preset for tactical analysis (post-match review - 20%)
## Shows all overlays and tactical information
func apply_preset_tactical_analysis() -> void:
	view_mode = ViewMode.FULL_LANDSCAPE

	# Ball animations
	enable_ball_trail = true
	enable_ball_height = true
	enable_ball_rotation = true
	enable_ball_squash_stretch = true
	enable_ball_bounce = true

	# Player animations
	enable_player_trails = true
	enable_player_shadows = true
	enable_player_stretch = true
	enable_hero_highlight = true

	# Event overlays (all on)
	show_pass_lines = true
	show_shot_points = true
	show_pressure_points = true
	show_run_segments = true
	show_dribble_segments = true
	show_throughball_lines = true
	show_cards = true
	show_assists = true

	# Heat map
	enable_heat_map = true

	# Minimap off (main view shows full field)
	show_minimap = false

	queue_redraw()


## Apply preset for minimap (deprecated compatibility)
## Minimal animations for performance
## @deprecated Use apply_preset_session_match() instead
func apply_preset_minimap() -> void:
	view_mode = ViewMode.FULL_PORTRAIT

	# Disable all animations for performance
	enable_ball_height = false
	enable_ball_squash_stretch = false
	enable_ball_trail = false
	enable_ball_rotation = false
	enable_ball_bounce = false

	enable_player_trails = false
	enable_player_stretch = false
	enable_player_shadows = false
	enable_hero_highlight = false

	# Disable all overlays
	show_pass_lines = false
	show_shot_points = false
	show_pressure_points = false
	show_run_segments = false
	show_dribble_segments = false
	show_throughball_lines = false
	show_cards = false
	show_assists = false
	enable_heat_map = false

	# No minimap
	show_minimap = false

	queue_redraw()


## Helper to update minimap position based on current view size
func _update_minimap_position_for_view() -> void:
	if show_minimap and get_size().x > 0:
		# Position minimap at top-right corner
		minimap_position.x = get_size().x - minimap_size.x - 16
		minimap_position.y = 16


#endregion


#region Process Loop
func _process(delta: float) -> void:
	var dirty: bool = false

	if _is_playing and not _events.is_empty():
		_current_time = min(_current_time + delta * _play_speed, _max_time)
		time_changed.emit(_current_time)

		while _event_index < _events.size():
			var event_time: float = _event_time(_events[_event_index])
			if event_time > _current_time:
				break
			if _apply_event(_events[_event_index]):
				dirty = true
			_event_index += 1

		if _event_index >= _events.size():
			_is_playing = false
			playback_finished.emit()

	# Update highlight timers
	if not _highlight_timers.is_empty():
		var remove_ids: Array = []
		for pid in _highlight_timers.keys():
			_highlight_timers[pid] -= delta
			if _highlight_timers[pid] <= 0.0:
				remove_ids.append(pid)
		for pid in remove_ids:
			_highlight_timers.erase(pid)
		if not remove_ids.is_empty():
			dirty = true

	# Update assist timers
	if not _assist_timers.is_empty():
		var erase_assists: Array = []
		for pid in _assist_timers.keys():
			_assist_timers[pid] -= delta
			if _assist_timers[pid] <= 0.0:
				erase_assists.append(pid)
		for pid in erase_assists:
			_assist_timers.erase(pid)
		if not erase_assists.is_empty():
			dirty = true

	# Update display positions (smooth interpolation)
	if _update_display_positions(delta):
		dirty = true

	# Ball animations
	if enable_ball_trail:
		_update_ball_trail(delta)
		if not _ball_trail_points.is_empty():
			dirty = true

	if enable_player_trails:
		_update_player_trails(delta)
		if not _player_trail_points.is_empty():
			dirty = true

	if enable_ball_bounce and _prev_ball_z > BALL_BOUNCE_THRESHOLD and _ball_z < 0.1:
		_play_ball_bounce()
	_prev_ball_z = _ball_z

	if enable_ball_rotation:
		_update_ball_rotation(delta)

	# Update camera follow position
	if view_mode == ViewMode.CAMERA_FOLLOW:
		if _update_camera_follow(delta):
			dirty = true

	if dirty:
		queue_redraw()


#endregion


#region Camera Follow
func _update_camera_follow(delta: float) -> bool:
	## Update camera position to follow ball with safe area constraints
	var changed := false

	# Calculate ball position in landscape canvas coordinates
	var landscape_size := _get_landscape_render_size()
	var ball_canvas_pos := _ball_to_landscape_canvas(_ball_display_position)

	# Camera viewport dimensions (portrait orientation)
	var cam_width := camera_viewport_size.x / camera_zoom
	var cam_height := camera_viewport_size.y / camera_zoom

	# Calculate target camera position (centered on ball with safe area)
	var safe_margin := camera_safe_margin / camera_zoom

	# Calculate bounds for camera target based on safe area
	var target_x := _camera_target.x
	var target_y := _camera_target.y

	# Check if ball is outside safe area and adjust target
	var cam_left := _camera_position.x - cam_width * 0.5
	var cam_right := _camera_position.x + cam_width * 0.5
	var cam_top := _camera_position.y - cam_height * 0.5
	var cam_bottom := _camera_position.y + cam_height * 0.5

	# Safe area boundaries
	var safe_left := cam_left + safe_margin.x
	var safe_right := cam_right - safe_margin.x
	var safe_top := cam_top + safe_margin.y
	var safe_bottom := cam_bottom - safe_margin.y

	# Adjust target if ball is outside safe area
	if ball_canvas_pos.x < safe_left:
		target_x = ball_canvas_pos.x + cam_width * 0.5 - safe_margin.x
	elif ball_canvas_pos.x > safe_right:
		target_x = ball_canvas_pos.x - cam_width * 0.5 + safe_margin.x

	if ball_canvas_pos.y < safe_top:
		target_y = ball_canvas_pos.y + cam_height * 0.5 - safe_margin.y
	elif ball_canvas_pos.y > safe_bottom:
		target_y = ball_canvas_pos.y - cam_height * 0.5 + safe_margin.y

	_camera_target = Vector2(target_x, target_y)

	# Clamp camera target to stay within field bounds
	var min_x := cam_width * 0.5
	var max_x := landscape_size.x - cam_width * 0.5
	var min_y := cam_height * 0.5
	var max_y := landscape_size.y - cam_height * 0.5

	_camera_target.x = clamp(_camera_target.x, min_x, max_x)
	_camera_target.y = clamp(_camera_target.y, min_y, max_y)

	# Smooth interpolation to target
	var lerp_factor: float = clamp(delta * camera_smooth_speed, 0.0, 1.0)
	var new_pos := _camera_position.lerp(_camera_target, lerp_factor)

	if new_pos.distance_to(_camera_position) > 0.1:
		_camera_position = new_pos
		changed = true

	return changed


func _ball_to_landscape_canvas(ball_pos: Vector2) -> Vector2:
	## Convert ball field position to landscape canvas coordinates
	var landscape_size := _get_landscape_render_size()
	var field_rect := Rect2(
		Vector2(FIELD_PADDING, FIELD_PADDING), landscape_size - Vector2(FIELD_PADDING * 2, FIELD_PADDING * 2)
	)

	var px: float = clamp(ball_pos.x / FIELD_LENGTH, 0.0, 1.0)
	var py: float = clamp(1.0 - (ball_pos.y / FIELD_WIDTH), 0.0, 1.0)

	return Vector2(field_rect.position.x + px * field_rect.size.x, field_rect.position.y + py * field_rect.size.y)


func get_camera_rect() -> Rect2:
	## Get current camera view rectangle in landscape coordinates
	var cam_width := camera_viewport_size.x / camera_zoom
	var cam_height := camera_viewport_size.y / camera_zoom
	return Rect2(_camera_position.x - cam_width * 0.5, _camera_position.y - cam_height * 0.5, cam_width, cam_height)


func reset_camera_to_ball() -> void:
	## Instantly reset camera position to ball location
	var ball_canvas := _ball_to_landscape_canvas(_ball_display_position)
	_camera_position = ball_canvas
	_camera_target = ball_canvas
	queue_redraw()


#endregion


#region Display Position Update
func _update_display_positions(delta: float) -> bool:
	var changed := false
	var player_lerp: float = clamp(delta * POSITION_SMOOTH_SPEED, 0.0, 1.0)

	for pid in _player_positions.keys():
		var target: Vector2 = _player_positions[pid]
		var current: Vector2 = _display_positions.get(pid, target)
		var updated: Vector2 = current.lerp(target, player_lerp)
		if updated.distance_to(current) > SMOOTH_EPSILON:
			_display_positions[pid] = updated
			changed = true
		elif current != target:
			_display_positions[pid] = target
			changed = true

	var cleanup: Array = []
	for pid in _display_positions.keys():
		if not _player_positions.has(pid):
			cleanup.append(pid)
	for pid in cleanup:
		_display_positions.erase(pid)
		changed = true

	var ball_lerp: float = clamp(delta * BALL_SMOOTH_SPEED, 0.0, 1.0)
	var next_ball := _ball_display_position.lerp(_ball_position, ball_lerp)
	if next_ball.distance_to(_ball_display_position) > SMOOTH_EPSILON:
		_ball_display_position = next_ball
		changed = true
	elif _ball_display_position != _ball_position:
		_ball_display_position = _ball_position
		changed = true

	if enable_ball_height:
		var target_radius: float = _get_ball_draw_radius(_ball_z)
		var radius_lerp: float = clamp(delta * 8.0, 0.0, 1.0)
		var next_radius: float = lerp(_ball_display_radius, target_radius, radius_lerp)
		if abs(next_radius - _ball_display_radius) > 0.01:
			_ball_display_radius = next_radius
			changed = true

	return changed


#endregion


#region Event Processing
func _apply_event(event: Dictionary) -> bool:
	var changed: bool = false
	var target: Variant = _extract_position(event)

	if target:
		var target_vec: Vector2 = Vector2(float(target.x), float(target.y))
		if target_vec != _ball_position:
			_ball_position = target_vec
			changed = true

		if enable_ball_height and target is Dictionary:
			var target_dict := target as Dictionary
			var new_z: float = 0.0
			if target_dict.has("z"):
				new_z = float(target_dict.get("z", 0.0))
			elif target_dict.has("height"):
				new_z = float(target_dict.get("height", 0.0))
			if abs(new_z - _ball_z) > 0.001:
				_ball_z = clamp(new_z, 0.0, 2.0)
				changed = true

	var pid := _resolve_event_player_track_id(event)
	if pid >= 0:
		_highlight_timers[pid] = HIGHLIGHT_DURATION
		changed = true

	var kind := str(event.get("kind", event.get("type", ""))).to_lower()
	match kind:
		"run":
			if show_run_segments:
				_handle_run_segment(event)
		"dribble":
			if show_dribble_segments:
				_handle_dribble_segment(event)
		"through_ball":
			if show_throughball_lines:
				_handle_throughball_event(event)
		"kick_off":
			_ball_position = Vector2(FIELD_LENGTH * 0.5, FIELD_WIDTH * 0.5)
			changed = true
		"shot":
			if show_shot_points:
				_handle_shot_event(event)
		"pass":
			if show_pass_lines:
				_handle_pass_event(event, false)
		"header":
			_handle_header_event(event)

	event_triggered.emit(event)
	return changed


func _extract_position(event: Dictionary) -> Variant:
	var keys := ["to", "ball_position", "position"]
	for key in keys:
		if event.has(key) and event.get(key) is Dictionary:
			return event.get(key)
	var base: Variant = event.get("base", {})
	if base is Dictionary and (base as Dictionary).has("position"):
		var base_pos: Variant = (base as Dictionary).get("position")
		if base_pos is Dictionary:
			return base_pos
	return null


#endregion


#region Drawing
func _draw() -> void:
	if view_mode == ViewMode.CAMERA_FOLLOW:
		_draw_camera_follow_view()
	else:
		_draw_full_view()


func _draw_full_view() -> void:
	## Draw the full field view (FULL_PORTRAIT or FULL_LANDSCAPE)
	var rect: Rect2 = _field_rect()

	# Background
	_draw_field_background(rect)
	draw_rect(rect, BORDER_COLOR, false, 2.0)

	# Field lines
	_draw_field_lines(rect)
	_draw_penalty_boxes(rect)

	# Heat map
	if enable_heat_map:
		_draw_goal_heatmap()

	# Event overlays
	if show_run_segments:
		_draw_run_segments()
	if show_dribble_segments:
		_draw_dribble_segments()
	if show_throughball_lines:
		_draw_throughball_lines()
	if show_pass_lines:
		_draw_pass_lines()
	if show_pressure_points:
		_draw_pressure_points()

	# Player trails
	if enable_player_trails:
		_draw_player_trails()

	# Players
	_draw_markers(_home_markers)
	_draw_markers(_away_markers)

	# Headers and communication
	_draw_header_events()
	_draw_communication_events()

	# Shots
	if show_shot_points:
		_draw_shot_points()

	# Ball trail
	if enable_ball_trail:
		_draw_ball_trail()

	# Ball
	_draw_ball()

	if show_dsa_overlay:
		_draw_dsa_overlay()


func _draw_camera_follow_view() -> void:
	## Draw the camera follow view with offset transform
	var cam_rect := get_camera_rect()
	var view_size := get_size()

	# Calculate scale to fit camera view into screen
	var scale_x := view_size.x / cam_rect.size.x
	var scale_y := view_size.y / cam_rect.size.y
	var scale_factor := minf(scale_x, scale_y)

	# Calculate offset to center the view
	var scaled_size := cam_rect.size * scale_factor
	var offset := (view_size - scaled_size) * 0.5

	# Apply transform: scale and translate
	var transform := Transform2D()
	transform = transform.scaled(Vector2(scale_factor, scale_factor))
	transform = transform.translated(-cam_rect.position)
	transform.origin += offset

	draw_set_transform_matrix(transform)

	# Now draw the full landscape field
	var rect: Rect2 = _field_rect()

	# Background
	_draw_field_background(rect)
	draw_rect(rect, BORDER_COLOR, false, 2.0)

	# Field lines
	_draw_field_lines(rect)
	_draw_penalty_boxes(rect)

	# Heat map
	if enable_heat_map:
		_draw_goal_heatmap()

	# Event overlays
	if show_run_segments:
		_draw_run_segments()
	if show_dribble_segments:
		_draw_dribble_segments()
	if show_throughball_lines:
		_draw_throughball_lines()
	if show_pass_lines:
		_draw_pass_lines()
	if show_pressure_points:
		_draw_pressure_points()

	# Player trails
	if enable_player_trails:
		_draw_player_trails()

	# Players
	_draw_markers(_home_markers)
	_draw_markers(_away_markers)

	# Headers and communication
	_draw_header_events()
	_draw_communication_events()

	# Shots
	if show_shot_points:
		_draw_shot_points()

	# Ball trail
	if enable_ball_trail:
		_draw_ball_trail()

	# Ball
	_draw_ball()

	# Reset transform
	draw_set_transform_matrix(Transform2D.IDENTITY)

	# Minimap overlay (drawn in screen space, after transform reset)
	if show_minimap:
		_draw_minimap_overlay()

	if show_dsa_overlay:
		_draw_dsa_overlay()


func _draw_minimap_overlay() -> void:
	## Draw a minimap showing the full field with current camera position indicator
	var mm_rect := Rect2(minimap_position, minimap_size)

	# Background with transparency
	var bg_color := Color(FIELD_COLOR.r, FIELD_COLOR.g, FIELD_COLOR.b, minimap_opacity)
	draw_rect(mm_rect, bg_color)

	# Border
	draw_rect(mm_rect, minimap_border_color, false, 2.0)

	# Calculate field rect within minimap (portrait orientation)
	var mm_inset := Vector2(4, 4)
	var mm_avail := minimap_size - mm_inset * 2.0
	var field_ratio := FIELD_WIDTH / FIELD_LENGTH  # Portrait ratio
	var mm_field_width := mm_avail.x
	var mm_field_height := mm_field_width / field_ratio
	if mm_field_height > mm_avail.y:
		mm_field_height = mm_avail.y
		mm_field_width = mm_field_height * field_ratio

	var mm_field_pos := (
		minimap_position + mm_inset + Vector2((mm_avail.x - mm_field_width) * 0.5, (mm_avail.y - mm_field_height) * 0.5)
	)
	var mm_field_rect := Rect2(mm_field_pos, Vector2(mm_field_width, mm_field_height))

	# Draw field lines (simplified)
	_draw_minimap_field_lines(mm_field_rect)

	# Draw camera view rectangle
	_draw_minimap_camera_rect(mm_field_rect)

	# Draw players (simplified)
	_draw_minimap_players(mm_field_rect)

	# Draw ball
	_draw_minimap_ball(mm_field_rect)


func _draw_minimap_field_lines(rect: Rect2) -> void:
	## Draw simplified field lines in minimap
	var line_color := Color(LINE_COLOR.r, LINE_COLOR.g, LINE_COLOR.b, 0.5)

	# Outer border
	draw_rect(rect, line_color, false, 1.0)

	# Center line
	var mid_y := rect.position.y + rect.size.y * 0.5
	draw_line(Vector2(rect.position.x, mid_y), Vector2(rect.position.x + rect.size.x, mid_y), line_color, 1.0)

	# Center circle (small)
	var center := rect.position + rect.size * 0.5
	var circle_radius := minf(rect.size.x, rect.size.y) * 0.08
	draw_arc(center, circle_radius, 0, TAU, 16, line_color, 1.0)


func _draw_minimap_camera_rect(mm_field_rect: Rect2) -> void:
	## Draw the current camera view area on the minimap
	var cam_rect := get_camera_rect()
	var landscape_size := _get_landscape_render_size()

	# Convert camera rect to field coordinates (landscape)
	var field_cam_x := cam_rect.position.x / landscape_size.x * FIELD_LENGTH
	var field_cam_y := (1.0 - (cam_rect.position.y + cam_rect.size.y) / landscape_size.y) * FIELD_WIDTH
	var field_cam_w := cam_rect.size.x / landscape_size.x * FIELD_LENGTH
	var field_cam_h := cam_rect.size.y / landscape_size.y * FIELD_WIDTH

	# Convert to minimap coordinates (portrait: X→Y, Y→X inverted)
	var mm_cam_x := mm_field_rect.position.x + (field_cam_y / FIELD_WIDTH) * mm_field_rect.size.x
	var mm_cam_y := mm_field_rect.position.y + (1.0 - (field_cam_x + field_cam_w) / FIELD_LENGTH) * mm_field_rect.size.y
	var mm_cam_w := (field_cam_h / FIELD_WIDTH) * mm_field_rect.size.x
	var mm_cam_h := (field_cam_w / FIELD_LENGTH) * mm_field_rect.size.y

	var cam_indicator_rect := Rect2(mm_cam_x, mm_cam_y, mm_cam_w, mm_cam_h)
	draw_rect(cam_indicator_rect, minimap_camera_rect_color, false, 2.0)


func _draw_minimap_players(mm_field_rect: Rect2) -> void:
	## Draw simplified player markers on minimap
	var player_radius := 3.0

	# Home team
	for marker in _home_markers:
		var field_pos: Vector2 = marker.get("field_pos", Vector2.ZERO)
		var screen_pos := _field_to_minimap(field_pos, mm_field_rect)
		var color := Color(HOME_COLOR.r, HOME_COLOR.g, HOME_COLOR.b, 0.9)
		draw_circle(screen_pos, player_radius, color)

	# Away team
	for marker in _away_markers:
		var field_pos: Vector2 = marker.get("field_pos", Vector2.ZERO)
		var screen_pos := _field_to_minimap(field_pos, mm_field_rect)
		var color := Color(AWAY_COLOR.r, AWAY_COLOR.g, AWAY_COLOR.b, 0.9)
		draw_circle(screen_pos, player_radius, color)


func _draw_minimap_ball(mm_field_rect: Rect2) -> void:
	## Draw ball on minimap
	var screen_pos := _field_to_minimap(_ball_display_position, mm_field_rect)
	draw_circle(screen_pos, 4.0, BALL_COLOR)


func _field_to_minimap(field_pos: Vector2, mm_field_rect: Rect2) -> Vector2:
	## Convert field position to minimap screen coordinates (portrait orientation)
	# Portrait: field X → minimap Y (inverted), field Y → minimap X
	var px: float = clamp(field_pos.y / FIELD_WIDTH, 0.0, 1.0)
	var py: float = clamp(1.0 - (field_pos.x / FIELD_LENGTH), 0.0, 1.0)
	return Vector2(
		mm_field_rect.position.x + px * mm_field_rect.size.x, mm_field_rect.position.y + py * mm_field_rect.size.y
	)


func _draw_field_background(rect: Rect2) -> void:
	draw_rect(rect, FIELD_COLOR)


func _draw_field_lines(rect: Rect2) -> void:
	# Center line
	var mid_y: float = rect.position.y + rect.size.y * 0.5
	draw_line(Vector2(rect.position.x, mid_y), Vector2(rect.position.x + rect.size.x, mid_y), LINE_COLOR, 1.5)

	# Center circle
	var center: Vector2 = rect.position + rect.size * 0.5
	var circle_radius: float = min(rect.size.x, rect.size.y) * 0.12
	draw_arc(center, circle_radius, 0, TAU, 64, LINE_COLOR, 1.5)


func _draw_penalty_boxes(rect: Rect2) -> void:
	var penalty_depth: float = rect.size.y * (PENALTY_LENGTH / FIELD_LENGTH)
	var penalty_span: float = rect.size.x * (PENALTY_WIDTH / FIELD_WIDTH)
	var offset_x: float = (rect.size.x - penalty_span) * 0.5

	var top_rect: Rect2 = Rect2(
		Vector2(rect.position.x + offset_x, rect.position.y), Vector2(penalty_span, penalty_depth)
	)
	var bottom_rect: Rect2 = Rect2(
		Vector2(rect.position.x + offset_x, rect.position.y + rect.size.y - penalty_depth),
		Vector2(penalty_span, penalty_depth)
	)
	draw_rect(top_rect, LINE_COLOR, false, 1.0)
	draw_rect(bottom_rect, LINE_COLOR, false, 1.0)


func _draw_ball() -> void:
	var screen_pos: Vector2 = _to_canvas(_ball_display_position)

	# P6: Ball height offset - ball moves UP on screen when airborne
	# Height multiplier: 20 pixels per meter of height (adjustable for visual effect)
	var height_offset := Vector2.ZERO
	if enable_ball_height and _ball_z > 0.01:
		height_offset = Vector2(0, -_ball_z * 20.0)  # Negative Y = up on screen

	# Shadow (stays at ground level, not offset)
	if enable_ball_height and _ball_z > 0.01:
		var shadow_offset: Vector2 = Vector2(2, 4) * _ball_z
		var shadow_alpha: float = 0.35 * (1.0 - _ball_z * 0.4)
		var shadow_scale: float = 1.0 - (_ball_z * 0.15)  # Shadow shrinks as ball goes higher
		draw_circle(screen_pos + shadow_offset, _ball_display_radius * 0.9 * shadow_scale, Color(0, 0, 0, shadow_alpha))

	# Ball body (offset by height)
	var ball_draw_pos := screen_pos + height_offset
	var radius: float = _ball_display_radius
	if enable_ball_squash_stretch:
		radius *= (_ball_squash_scale.x + _ball_squash_scale.y) * 0.5

	var ball_color := BALL_COLOR
	if enable_ball_squash_stretch:
		ball_color = Color(
			min(1.0, BALL_COLOR.r * _ball_brightness),
			min(1.0, BALL_COLOR.g * _ball_brightness),
			min(1.0, BALL_COLOR.b * _ball_brightness),
			BALL_COLOR.a
		)

	draw_circle(ball_draw_pos, radius, ball_color)

	# Rotation indicator (also offset by height)
	if enable_ball_rotation and abs(_ball_rotation) > 0.01:
		var indicator_offset := Vector2(cos(_ball_rotation), sin(_ball_rotation)) * radius * 0.5
		draw_circle(ball_draw_pos + indicator_offset, radius * 0.2, Color(0.8, 0.8, 0.8, 0.5))


func _draw_markers(markers: Array) -> void:
	for marker in markers:
		if not (marker is Dictionary):
			continue
		var marker_dict: Dictionary = marker as Dictionary
		var pid := int(marker_dict.get("id", 0))
		var default_pos: Vector2 = marker_dict.get("field_pos", Vector2(FIELD_LENGTH * 0.5, FIELD_WIDTH * 0.5))
		var pos: Vector2 = default_pos
		if _display_positions.has(pid):
			pos = _display_positions[pid]
		elif _player_positions.has(pid):
			pos = _player_positions[pid]
		var color: Color = marker_dict.get("color", HOME_COLOR)
		var screen_pos: Vector2 = _to_canvas(pos)

		# Shadow
		if enable_player_shadows:
			draw_ellipse(
				screen_pos + Vector2(0, 3), Vector2(PLAYER_SIZE.x * 0.6, PLAYER_SIZE.y * 0.3), Color(0, 0, 0, 0.25)
			)

		# Player body
		var rect: Rect2 = Rect2(screen_pos - PLAYER_SIZE * 0.5, PLAYER_SIZE)
		draw_rect(rect, color, true)

		# Highlight
		if _highlight_timers.has(pid):
			var strength: float = clamp(_highlight_timers[pid] / HIGHLIGHT_DURATION, 0.0, 1.0)
			var alpha: float = clamp(0.25 + 0.5 * strength, 0.0, 1.0)
			var highlight_color: Color = Color(color.r, color.g, color.b, alpha)
			draw_rect(rect.grow(4.0), highlight_color, false, 2.0)

		# Hero highlight
		if enable_hero_highlight and _hero_player_id != "" and str(pid) == _hero_player_id:
			draw_rect(rect.grow(6.0), Color(1.0, 0.85, 0.2, 0.7), false, 3.0)

		# Assist label
		if show_assists and _assist_timers.has(pid):
			var font: Font = get_theme_default_font()
			var font_size: int = get_theme_default_font_size()
			var label_pos := screen_pos + Vector2(-PLAYER_SIZE.x * 0.5, -PLAYER_SIZE.y * 0.4)
			draw_string(font, label_pos, "A", HORIZONTAL_ALIGNMENT_LEFT, -1, font_size, Color(1.0, 0.95, 0.4))

		# Card status
		if show_cards and _card_status.has(pid):
			var card := str(_card_status.get(pid, ""))
			var card_color := Color(1, 1, 0, 0.95) if card == "yellow" else Color(1, 0.2, 0.2, 0.95)
			var font: Font = get_theme_default_font()
			var font_size: int = get_theme_default_font_size()
			var label := "Y" if card == "yellow" else "R"
			var card_pos := screen_pos + Vector2(PLAYER_SIZE.x * 0.25, -PLAYER_SIZE.y * 0.5)
			draw_rect(Rect2(card_pos - Vector2(4, font_size * 0.5), Vector2(12, font_size)), card_color, true)
			draw_string(
				font,
				card_pos + Vector2(-2, font_size * 0.3),
				label,
				HORIZONTAL_ALIGNMENT_LEFT,
				-1,
				font_size - 2,
				Color(0, 0, 0)
			)

		# P6: Stamina bar (above player)
		if show_stamina_bars and _player_stamina.has(pid):
			var stamina: float = _player_stamina[pid]
			var bar_pos := screen_pos + Vector2(-stamina_bar_size.x * 0.5, -PLAYER_SIZE.y - stamina_bar_size.y - 2)
			# Background (dark)
			draw_rect(Rect2(bar_pos, stamina_bar_size), Color(0.2, 0.2, 0.2, 0.7), true)
			# Foreground (green to red based on stamina)
			var bar_fill_width: float = stamina_bar_size.x * stamina
			var fill_color := (
				Color(0.2, 0.85, 0.3)
				if stamina > 0.5
				else Color(0.9, 0.7, 0.1) if stamina > 0.25 else Color(0.95, 0.2, 0.2)
			)
			draw_rect(Rect2(bar_pos, Vector2(bar_fill_width, stamina_bar_size.y)), fill_color, true)
			# Border
			draw_rect(Rect2(bar_pos, stamina_bar_size), Color(0.5, 0.5, 0.5, 0.8), false, 1.0)


func draw_ellipse(center: Vector2, radius: Vector2, color: Color) -> void:
	var points: PackedVector2Array = PackedVector2Array()
	var segments: int = 16
	for i in range(segments + 1):
		var angle: float = (float(i) / float(segments)) * TAU
		points.append(center + Vector2(cos(angle) * radius.x, sin(angle) * radius.y))
	draw_colored_polygon(points, color)


#endregion


#region Trail Drawing
func _draw_ball_trail() -> void:
	if _ball_trail_points.is_empty():
		return
	for i in range(_ball_trail_points.size() - 1):
		var point_a: Dictionary = _ball_trail_points[i]
		var point_b: Dictionary = _ball_trail_points[i + 1]
		var alpha_a: float = float(point_a.get("alpha", 0.0))
		var alpha_b: float = float(point_b.get("alpha", 0.0))
		if alpha_a < 0.05 and alpha_b < 0.05:
			continue
		var pos_a: Vector2 = _to_canvas(point_a.get("pos", Vector2.ZERO))
		var pos_b: Vector2 = _to_canvas(point_b.get("pos", Vector2.ZERO))
		var color: Color = Color(1.0, 0.9, 0.7, (alpha_a + alpha_b) * 0.5 * 0.6)
		draw_line(pos_a, pos_b, color, 2.0)


func _draw_player_trails() -> void:
	for pid in _player_trail_points.keys():
		var trail: Array = _player_trail_points[pid]
		if trail.size() < 2:
			continue
		for i in range(trail.size() - 1):
			var point_a: Dictionary = trail[i]
			var point_b: Dictionary = trail[i + 1]
			var alpha: float = float(point_a.get("alpha", 0.0))
			if alpha < 0.1:
				continue
			var pos_a: Vector2 = _to_canvas(point_a.get("pos", Vector2.ZERO))
			var pos_b: Vector2 = _to_canvas(point_b.get("pos", Vector2.ZERO))
			var color: Color = Color(0.5, 0.7, 1.0, alpha * 0.4)
			draw_line(pos_a, pos_b, color, 1.5)


#endregion


#region Event Overlay Drawing
func _draw_pass_lines() -> void:
	if _pass_lines.is_empty():
		return
	for line in _pass_lines:
		if not (line is Dictionary):
			continue
		var from_pos: Variant = line.get("from", null)
		var to_pos: Variant = line.get("to", null)
		if not (from_pos is Vector2) or not (to_pos is Vector2):
			continue
		var screen_from := _to_canvas(from_pos)
		var screen_to := _to_canvas(to_pos)
		var team_side := int(line.get("team", 0))
		var color := _team_color(team_side, PASS_COLOR.a)
		draw_line(screen_from, screen_to, color, 1.5)
		_draw_arrow_cap(screen_from, screen_to, color, 1.0)


func _draw_shot_points() -> void:
	if _shot_points.is_empty():
		return
	for shot in _shot_points:
		if not (shot is Dictionary):
			continue
		var pos: Variant = shot.get("pos", null)
		if not (pos is Vector2):
			continue
		var screen_pos := _to_canvas(pos)
		var xg: float = float(shot.get("xg", 0.1))
		var radius: float = 8.0 + xg * 20.0
		var team_side := int(shot.get("team", 0))
		var color := _team_color(team_side, SHOT_COLOR.a)
		draw_circle(screen_pos, radius, Color(color.r, color.g, color.b, 0.3))
		draw_arc(screen_pos, radius, 0, TAU, 32, color, 2.0)

		var font: Font = get_theme_default_font()
		var font_size: int = max(get_theme_default_font_size() - 2, 9)
		draw_string(
			font, screen_pos + Vector2(radius + 4, 4), "%.2f xG" % xg, HORIZONTAL_ALIGNMENT_LEFT, -1, font_size, color
		)


func _draw_pressure_points() -> void:
	if _pressure_points.is_empty():
		return
	for point in _pressure_points:
		if not (point is Dictionary):
			continue
		var pos: Variant = point.get("pos", null)
		if not (pos is Vector2):
			continue
		var screen_pos := _to_canvas(pos)
		var team_side := int(point.get("team", 0))
		var color := _team_color(team_side, 0.5)
		draw_circle(screen_pos, 5.0, color)


func _draw_run_segments() -> void:
	if _run_segments.is_empty():
		return
	for segment in _run_segments:
		if not (segment is Dictionary):
			continue
		var from_pos: Variant = segment.get("from", null)
		var to_pos: Variant = segment.get("to", null)
		if not (from_pos is Vector2) or not (to_pos is Vector2):
			continue
		var screen_from := _to_canvas(from_pos)
		var screen_to := _to_canvas(to_pos)
		var with_ball: bool = segment.get("with_ball", false)
		var color := RUN_WITH_BALL_COLOR if with_ball else RUN_COLOR
		var width := 2.6 if with_ball else 1.8
		draw_line(screen_from, screen_to, color, width)
		_draw_arrow_cap(screen_from, screen_to, color, width * 0.7)


func _draw_dribble_segments() -> void:
	if _dribble_segments.is_empty():
		return
	for segment in _dribble_segments:
		if not (segment is Dictionary):
			continue
		var from_pos: Variant = segment.get("from", null)
		var to_pos: Variant = segment.get("to", null)
		if not (from_pos is Vector2) or not (to_pos is Vector2):
			continue
		var screen_from := _to_canvas(from_pos)
		var screen_to := _to_canvas(to_pos)
		var team_side := int(segment.get("team", 0))
		var color := _team_color(team_side, DRIBBLE_COLOR.a)
		_draw_dashed_line(screen_from, screen_to, color, 1.8)


func _draw_throughball_lines() -> void:
	if _throughball_lines.is_empty():
		return
	for line in _throughball_lines:
		if not (line is Dictionary):
			continue
		var from_pos: Variant = line.get("from", null)
		var to_pos: Variant = line.get("to", null)
		if not (from_pos is Vector2) or not (to_pos is Vector2):
			continue
		var screen_from := _to_canvas(from_pos)
		var screen_to := _to_canvas(to_pos)
		var team_side := int(line.get("team", 0))
		var color := _team_color(team_side, THROUGH_BALL_COLOR.a)
		_draw_dashed_line(screen_from, screen_to, color, 2.0)
		_draw_arrow_cap(screen_from, screen_to, color, 1.4)


func _draw_header_events() -> void:
	if _header_events.is_empty():
		return
	for header in _header_events:
		if not (header is Dictionary):
			continue
		var pos: Variant = header.get("pos", null)
		if not (pos is Vector2):
			continue
		var screen_pos := _to_canvas(pos)
		var team_side := int(header.get("team", 0))
		var color := _team_color(team_side, 0.7)

		# Draw header icon (football shape with direction indicator)
		var radius := 7.0
		draw_circle(screen_pos, radius, Color(color.r, color.g, color.b, 0.3))
		draw_arc(screen_pos, radius, 0, TAU, 24, color, 1.5)

		# Draw direction arrow if available
		var direction: Variant = header.get("direction", null)
		if direction is Vector2:
			var dir_vec: Vector2 = direction as Vector2
			var dir_normalized: Vector2 = dir_vec.normalized()
			var arrow_end: Vector2 = screen_pos + dir_normalized * (radius + 8.0)
			draw_line(screen_pos, arrow_end, color, 2.0)
			_draw_arrow_cap(screen_pos, arrow_end, color, 1.2)


func _draw_communication_events() -> void:
	if _communication_events.is_empty():
		return
	for comm in _communication_events:
		if not (comm is Dictionary):
			continue
		var pos: Variant = comm.get("pos", null)
		if not (pos is Vector2):
			continue
		var screen_pos := _to_canvas(pos)
		var team_side := int(comm.get("team", 0))
		var color := _team_color(team_side, 0.6)

		# Draw communication bubble
		var radius := 6.0
		draw_circle(screen_pos, radius, Color(color.r, color.g, color.b, 0.2))
		draw_arc(screen_pos, radius, 0, TAU, 20, color, 1.5)

		# Draw small dots to indicate speech
		var dot_offset := radius + 3.0
		for i in range(3):
			var dot_pos := screen_pos + Vector2(i * 2.5 - 2.5, -dot_offset)
			var dot_size := 1.5 - i * 0.3
			draw_circle(dot_pos, dot_size, color)

		# Draw line to target if targeted communication
		var has_target: bool = comm.get("has_target", false)
		var target_pos: Variant = comm.get("target_pos", null)
		if has_target and target_pos is Vector2:
			var screen_target := _to_canvas(target_pos)
			_draw_dashed_line(screen_pos, screen_target, Color(color.r, color.g, color.b, 0.4), 1.0)


func _draw_goal_heatmap() -> void:
	if not enable_heat_map:
		return
	var rect := _field_rect()
	var penalty_depth: float = rect.size.y * (PENALTY_LENGTH / FIELD_LENGTH)
	var penalty_span: float = rect.size.x * (PENALTY_WIDTH / FIELD_WIDTH)
	var offset_x: float = (rect.size.x - penalty_span) * 0.5

	for goal_side in [0, 1]:
		var zone_y: float = rect.position.y if goal_side == 0 else rect.position.y + rect.size.y - penalty_depth
		var zone_rect: Rect2 = Rect2(Vector2(rect.position.x + offset_x, zone_y), Vector2(penalty_span, penalty_depth))
		var intensity: float = _goal_heat_intensity(goal_side)
		if intensity > 0.0:
			var base_color: Color = HOME_COLOR if goal_side == 0 else AWAY_COLOR
			var alpha: float = (0.08 + intensity * 0.28) * heat_map_opacity
			var fill_color: Color = Color(base_color.r, base_color.g, base_color.b, alpha)
			draw_rect(zone_rect, fill_color, true)

	for goal_side in _goal_heat_samples.keys():
		var samples: Array = _goal_heat_samples.get(goal_side, [])
		for sample in samples:
			if not (sample is Dictionary):
				continue
			var field_pos: Variant = sample.get("pos", null)
			if not (field_pos is Vector2):
				continue
			var weight: float = clamp(float(sample.get("weight", 0.2)), 0.05, 1.3)
			var canvas_pos: Vector2 = _to_canvas(field_pos)
			var radius: float = 14.0 + weight * 30.0
			var base_color: Color = HOME_COLOR if goal_side == 0 else AWAY_COLOR
			var color: Color = Color(base_color.r, base_color.g, base_color.b, 0.18 * heat_map_opacity * weight)
			draw_circle(canvas_pos, radius, color)


func _draw_arrow_cap(from: Vector2, to: Vector2, color: Color, size: float) -> void:
	var direction: Vector2 = (to - from).normalized()
	if direction.length() < 0.01:
		return
	var perpendicular: Vector2 = Vector2(-direction.y, direction.x)
	var arrow_size: float = 8.0 * size
	var point1: Vector2 = to - direction * arrow_size + perpendicular * arrow_size * 0.5
	var point2: Vector2 = to - direction * arrow_size - perpendicular * arrow_size * 0.5
	draw_polygon([to, point1, point2], [color])


func _draw_dashed_line(from: Vector2, to: Vector2, color: Color, width: float) -> void:
	var direction: Vector2 = to - from
	var length: float = direction.length()
	if length < 1.0:
		return
	direction = direction.normalized()
	var dash_length: float = 8.0
	var gap_length: float = 4.0
	var current: float = 0.0
	while current < length:
		var start_pos: Vector2 = from + direction * current
		var end_pos: Vector2 = from + direction * min(current + dash_length, length)
		draw_line(start_pos, end_pos, color, width)
		current += dash_length + gap_length


#endregion


#region Coordinate Transformation
func _is_portrait_view() -> bool:
	## Helper to determine if current view should use portrait orientation
	match view_mode:
		ViewMode.FULL_PORTRAIT:
			return true
		ViewMode.FULL_LANDSCAPE:
			return false
		ViewMode.CAMERA_FOLLOW:
			return false  # Internal render is always landscape
		_:
			return portrait_mode  # Fallback to deprecated var


func _field_rect() -> Rect2:
	var view_size: Vector2 = get_size()

	# For CAMERA_FOLLOW, calculate landscape size for internal rendering
	if view_mode == ViewMode.CAMERA_FOLLOW:
		view_size = _get_landscape_render_size()

	var inset: Vector2 = Vector2(FIELD_PADDING, FIELD_PADDING)
	var avail: Vector2 = view_size - inset * 2.0
	if avail.x <= 0.0 or avail.y <= 0.0:
		return Rect2(Vector2.ZERO, view_size)

	var is_portrait: bool = _is_portrait_view()
	var ratio: float
	if is_portrait:
		ratio = FIELD_WIDTH / FIELD_LENGTH  # 68/105 ≈ 0.648
	else:
		ratio = FIELD_LENGTH / FIELD_WIDTH  # 105/68 ≈ 1.544

	var width: float
	var height: float
	if is_portrait:
		height = avail.y
		width = height * ratio
		if width > avail.x:
			width = avail.x
			height = width / ratio
	else:
		width = avail.x
		height = width / ratio
		if height > avail.y:
			height = avail.y
			width = height * ratio

	var pos := inset + Vector2((avail.x - width) * 0.5, (avail.y - height) * 0.5)
	return Rect2(pos, Vector2(width, height))


func _get_landscape_render_size() -> Vector2:
	## Calculate the full landscape render size based on zoom level
	var base_height: float = camera_viewport_size.y
	var aspect_ratio: float = FIELD_LENGTH / FIELD_WIDTH  # 105/68 ≈ 1.544
	var base_width: float = base_height * aspect_ratio
	return Vector2(base_width * camera_zoom, base_height * camera_zoom)


func _to_canvas(field_pos: Vector2) -> Vector2:
	var rect: Rect2 = _field_rect()
	var px_ratio: float
	var py_ratio: float

	if _is_portrait_view():
		# Portrait: X maps to screen X, Y maps to screen Y (inverted)
		px_ratio = clamp(field_pos.y / FIELD_WIDTH, 0.0, 1.0)
		py_ratio = clamp(1.0 - (field_pos.x / FIELD_LENGTH), 0.0, 1.0)
	else:
		# Landscape: standard mapping
		px_ratio = clamp(field_pos.x / FIELD_LENGTH, 0.0, 1.0)
		py_ratio = clamp(1.0 - (field_pos.y / FIELD_WIDTH), 0.0, 1.0)

	return Vector2(rect.position.x + px_ratio * rect.size.x, rect.position.y + py_ratio * rect.size.y)


#endregion


#region Ball Animation Helpers
func _get_ball_draw_radius(ball_z: float) -> float:
	var base_radius := BALL_RADIUS
	var height_factor := 1.0 + (ball_z * 0.5)
	return base_radius * height_factor


func _update_ball_trail(delta: float) -> void:
	# Add new point if ball is moving
	if _ball_trail_active or _ball_display_position.distance_to(_ball_position) > 0.5:
		if (
			_ball_trail_points.is_empty()
			or (_ball_trail_points[-1].get("pos", Vector2.ZERO) as Vector2).distance_to(_ball_display_position) > 1.0
		):
			_ball_trail_points.append({"pos": _ball_display_position, "alpha": 1.0})

	# Fade out points
	var to_remove: Array = []
	for i in range(_ball_trail_points.size()):
		_ball_trail_points[i]["alpha"] = float(_ball_trail_points[i].get("alpha", 1.0)) - delta * BALL_TRAIL_FADE_SPEED
		if float(_ball_trail_points[i].get("alpha", 0.0)) <= 0.0:
			to_remove.append(i)

	for i in range(to_remove.size() - 1, -1, -1):
		_ball_trail_points.remove_at(to_remove[i])

	# Limit points
	while _ball_trail_points.size() > BALL_TRAIL_MAX_POINTS:
		_ball_trail_points.remove_at(0)


func _update_player_trails(delta: float) -> void:
	for pid in _display_positions.keys():
		var pos: Vector2 = _display_positions[pid]
		if not _player_trail_points.has(pid):
			_player_trail_points[pid] = []
		var trail: Array = _player_trail_points[pid]

		# Add new point
		if trail.is_empty() or (trail[-1].get("pos", Vector2.ZERO) as Vector2).distance_to(pos) > 0.5:
			trail.append({"pos": pos, "alpha": 0.8})

		# Fade out
		var to_remove: Array = []
		for i in range(trail.size()):
			trail[i]["alpha"] = float(trail[i].get("alpha", 0.8)) - delta * 2.0
			if float(trail[i].get("alpha", 0.0)) <= 0.0:
				to_remove.append(i)

		for i in range(to_remove.size() - 1, -1, -1):
			trail.remove_at(to_remove[i])

		while trail.size() > PLAYER_TRAIL_MAX_POINTS:
			trail.remove_at(0)

		_player_trail_points[pid] = trail


func _update_ball_rotation(delta: float) -> void:
	var rotation_multiplier: float = 1.0
	match _current_height_profile:
		"lob":
			rotation_multiplier = 0.5 + _ball_z * 0.5
		"driven":
			rotation_multiplier = 2.0
		"header":
			rotation_multiplier = 1.2
	_ball_rotation += _ball_rotation_speed * rotation_multiplier * delta
	if abs(_ball_rotation) > TAU:
		_ball_rotation = fmod(_ball_rotation, TAU)


func _play_ball_bounce() -> void:
	if enable_ball_squash_stretch and not _ball_squash_animating:
		_play_bounce_squash_stretch()


func _play_bounce_squash_stretch() -> void:
	if _ball_squash_animating:
		return
	_ball_squash_animating = true
	_ball_squash_scale = Vector2(1.3, 0.7)
	queue_redraw()

	var tween := create_tween()
	tween.tween_property(self, "_ball_squash_scale", Vector2(1.0, 1.0), 0.15)
	tween.tween_callback(func(): _ball_squash_animating = false)


func _activate_ball_trail(_shot_type: String = "normal") -> void:
	_ball_trail_active = true


func _deactivate_ball_trail() -> void:
	_ball_trail_active = false


#endregion


#region Event Handlers
func _handle_pass_event(event: Dictionary, _is_clearance: bool = false) -> void:
	var from_id: int = int(event.get("from_player_id", event.get("player_id", 0)))
	var to_id: int = int(event.get("receiver_id", event.get("to_player_id", 0)))
	var destination: Variant = _target_variant_to_field(event.get("to", event.get("target", null)))
	var origin: Variant = _target_variant_to_field(event.get("from", null))

	if origin == null:
		var known: Variant = _get_player_position(from_id)
		if known is Vector2:
			origin = known
	if destination == null:
		var known: Variant = _get_player_position(to_id)
		if known is Vector2:
			destination = known

	if origin is Vector2 and destination is Vector2:
		(
			_pass_lines
			. append(
				{
					"from": origin,
					"to": destination,
					"team": _map_engine_team(event.get("team_id", 0)),
				}
			)
		)
		_limit_overlay_entries(_pass_lines)


func _handle_shot_event(event: Dictionary) -> void:
	var player_id: int = int(event.get("player_id", 0))
	var origin: Variant = _get_player_position(player_id)
	if origin == null:
		origin = _target_variant_to_field(event.get("position", event.get("target", null)))
	if not (origin is Vector2):
		return

	var xg_value := float(event.get("xg", event.get("xg_value", 0.1)))
	var team_side := _map_engine_team(event.get("team_id", 0))

	(
		_shot_points
		. append(
			{
				"pos": origin,
				"team": team_side,
				"xg": xg_value,
			}
		)
	)
	_limit_overlay_entries(_shot_points)

	_record_goal_heat_sample(1 - team_side, origin, xg_value)

	if enable_ball_trail:
		_activate_ball_trail("shot")


func _handle_run_segment(event: Dictionary) -> void:
	var from_pos: Variant = _target_variant_to_field(event.get("from", null))
	var to_pos: Variant = _target_variant_to_field(event.get("to", null))
	if not (from_pos is Vector2) or not (to_pos is Vector2):
		return
	(
		_run_segments
		. append(
			{
				"from": from_pos,
				"to": to_pos,
				"team": _map_engine_team(event.get("team_id", 0)),
				"with_ball": bool(event.get("with_ball", false)),
			}
		)
	)
	_limit_overlay_entries(_run_segments)


func _handle_dribble_segment(event: Dictionary) -> void:
	var from_pos: Variant = _target_variant_to_field(event.get("from", null))
	var to_pos: Variant = _target_variant_to_field(event.get("to", null))
	if not (from_pos is Vector2) or not (to_pos is Vector2):
		return
	(
		_dribble_segments
		. append(
			{
				"from": from_pos,
				"to": to_pos,
				"team": _map_engine_team(event.get("team_id", 0)),
			}
		)
	)
	_limit_overlay_entries(_dribble_segments)


func _handle_throughball_event(event: Dictionary) -> void:
	var from_pos: Variant = _target_variant_to_field(event.get("from", null))
	var to_pos: Variant = _target_variant_to_field(event.get("to", null))
	if not (from_pos is Vector2) or not (to_pos is Vector2):
		return
	(
		_throughball_lines
		. append(
			{
				"from": from_pos,
				"to": to_pos,
				"team": _map_engine_team(event.get("team_id", 0)),
			}
		)
	)
	_limit_overlay_entries(_throughball_lines)


func _handle_header_event(_event: Dictionary) -> void:
	if enable_ball_trail:
		_activate_ball_trail("header")
	if enable_ball_squash_stretch:
		_play_bounce_squash_stretch()


#endregion


#region Heat Map
func _record_goal_heat_sample(goal_side: int, field_pos: Variant, weight: float) -> void:
	if _goal_heat_locked or not (field_pos is Vector2):
		return
	var pos: Vector2 = field_pos
	if weight < 0.0:
		weight = 0.25
	var clamped_weight: float = clamp(weight, 0.05, 1.2)
	if not _goal_heat_samples.has(goal_side):
		_goal_heat_samples[goal_side] = []
	var samples: Array = _goal_heat_samples[goal_side]
	samples.append({"pos": pos, "weight": clamped_weight})
	_goal_heat_totals[goal_side] = _goal_heat_totals.get(goal_side, 0.0) + clamped_weight
	while samples.size() > MAX_HEAT_SAMPLES:
		var removed: Variant = samples[0]
		samples.remove_at(0)
		if removed is Dictionary and removed.has("weight"):
			_goal_heat_totals[goal_side] = max(
				0.0, _goal_heat_totals.get(goal_side, 0.0) - float(removed.get("weight", 0.0))
			)
	_goal_heat_samples[goal_side] = samples


func _goal_heat_intensity(goal_side: int) -> float:
	var total: float = _goal_heat_totals.get(goal_side, 0.0)
	if total <= 0.0:
		return 0.0
	return clamp(total / GOAL_HEAT_REFERENCE_XG, 0.0, 1.0)


#endregion


#region Marker Building
func _build_markers() -> void:
	_player_positions.clear()
	_player_identifier_to_marker.clear()
	_player_id_to_name.clear()
	_card_status.clear()
	_display_positions.clear()
	_home_markers = _create_marker_list(_rosters.get("home", {}), true)
	_away_markers = _create_marker_list(_rosters.get("away", {}), false)
	_register_marker_positions(_home_markers)
	_register_marker_positions(_away_markers)
	_sync_display_positions()


func _create_marker_list(roster: Variant, is_home: bool) -> Array:
	var players: Array = []
	if roster is Dictionary:
		var roster_dict: Dictionary = roster as Dictionary
		var roster_players: Variant = roster_dict.get("players", [])
		if roster_players is Array:
			for entry in roster_players:
				if entry is Dictionary:
					players.append(entry)

	if players.is_empty():
		for i in range(11):
			# Match OS default track_id layout: 0..10 home, 11..21 away
			var fallback_track_id := i if is_home else (11 + i)
			var fake_player := {
				"track_id": fallback_track_id,
				"id": fallback_track_id,
				"name": "Player %d" % (i + 1),
				"position": _fallback_position_for_index(i)
			}
			players.append(fake_player)

	var markers: Array = []
	for i in range(players.size()):
		var marker: Dictionary = _make_marker(players[i], i, players.size(), is_home)
		markers.append(marker)
	return markers


func _make_marker(player: Dictionary, index: int, _total: int, is_home: bool) -> Dictionary:
	# Prefer Match OS SSOT identity when available.
	var pid: int = int(player.get("track_id", player.get("id", (1000 + index) if is_home else (-1000 - index))))
	var position_name: String = str(player.get("position", "CM"))
	var field_pos: Vector2 = _resolve_field_position(position_name, is_home)
	return {
		"id": pid,
		"name": str(player.get("name", "Player")),
		"field_pos": field_pos,
		"color": HOME_COLOR if is_home else AWAY_COLOR
	}


func _resolve_field_position(position_name: String, is_home: bool) -> Vector2:
	var key: String = _match_position_key(position_name)
	var normalized: Vector2 = POSITION_NORMALS.get(key, Vector2(0.4, 0.5))
	var x_value: float = normalized.x
	if not is_home:
		x_value = 1.0 - x_value
	return Vector2(x_value * FIELD_LENGTH, normalized.y * FIELD_WIDTH)


func _match_position_key(position_name: String) -> String:
	var upper: String = position_name.strip_edges().to_upper()
	if upper.is_empty():
		return "CM"
	if "GK" in upper:
		return "GK"
	if "LB" in upper:
		return "LB"
	if "RB" in upper:
		return "RB"
	if "CB" in upper:
		return "CB"
	if "DM" in upper:
		return "DM"
	if "CM" in upper:
		return "CM"
	if "AM" in upper or "CAM" in upper:
		return "AM"
	if "LW" in upper:
		return "LW"
	if "RW" in upper:
		return "RW"
	if "ST" in upper or "FW" in upper:
		return "ST"
	return "CM"


func _register_marker_positions(markers: Array) -> void:
	for marker in markers:
		if not (marker is Dictionary):
			continue
		var marker_dict: Dictionary = marker
		var pid: int = int(marker_dict.get("id", 0))
		if pid < 0:
			continue
		var base_pos: Variant = marker_dict.get("field_pos", Vector2(FIELD_LENGTH * 0.5, FIELD_WIDTH * 0.5))
		var resolved_pos: Vector2 = base_pos if base_pos is Vector2 else Vector2(FIELD_LENGTH * 0.5, FIELD_WIDTH * 0.5)
		_player_positions[pid] = resolved_pos
		_display_positions[pid] = resolved_pos
		var readable_name := str(marker_dict.get("name", "")).strip_edges()
		if readable_name != "":
			_player_id_to_name[pid] = readable_name
		_player_identifier_to_marker[str(pid)] = pid


func _sync_display_positions() -> void:
	_display_positions.clear()
	for pid in _player_positions.keys():
		_display_positions[pid] = _player_positions[pid]


func _fallback_position_for_index(index: int) -> String:
	var defaults := ["GK", "LB", "LCB", "RCB", "RB", "DM", "LCM", "RCM", "LAM", "RAM", "ST"]
	return defaults[index] if index < defaults.size() else "CM"


#endregion


#region Utility Functions
func _resolve_event_player_track_id(event: Dictionary) -> int:
	# Event SSOT: identity is track_id-only. Name-based mapping is forbidden.
	if event.has("player_track_id"):
		return int(event.get("player_track_id", -1))
	var base: Variant = event.get("base", {})
	if base is Dictionary and (base as Dictionary).has("player_track_id"):
		return int((base as Dictionary).get("player_track_id", -1))
	return -1


func _resolve_marker_for_snapshot_key(raw_key: Variant) -> int:
	var key_str := str(raw_key).strip_edges()
	if key_str != "":
		var lookup_key := key_str.to_lower()
		if _player_identifier_to_marker.has(lookup_key):
			return int(_player_identifier_to_marker[lookup_key])
		if key_str.is_valid_int():
			var pid := int(key_str)
			if _player_positions.has(pid):
				return pid
	return -1


func _get_player_position(player_id: int) -> Variant:
	if player_id >= 0 and _player_positions.has(player_id):
		return _player_positions[player_id]
	return null


func _target_variant_to_field(target_variant: Variant) -> Variant:
	if target_variant == null:
		return null
	var x_val: float = FIELD_LENGTH * 0.5
	var y_val: float = FIELD_WIDTH * 0.5
	if target_variant is Dictionary:
		var dict: Dictionary = target_variant
		x_val = float(dict.get("x", dict.get("0", x_val)))
		if dict.has("z"):
			y_val = float(dict.get("z", y_val))
		else:
			y_val = float(dict.get("y", y_val))
	elif target_variant is Array:
		var arr: Array = target_variant
		if arr.size() >= 1:
			x_val = float(arr[0])
		if arr.size() >= 3:
			y_val = float(arr[2])
		elif arr.size() >= 2:
			y_val = float(arr[1])
	else:
		return null
	return Vector2(clamp(x_val, 0.0, FIELD_LENGTH), clamp(y_val, 0.0, FIELD_WIDTH))


func _vector_from_variant(value: Variant) -> Vector2:
	if value is Vector2:
		return value
	if value is Dictionary:
		var dict: Dictionary = value
		var x_val := float(dict.get("x", dict.get("0", 0.0)))
		var y_val := float(dict.get("y", dict.get("1", dict.get("z", 0.0))))
		return Vector2(x_val, y_val)
	return Vector2(FIELD_LENGTH * 0.5, FIELD_WIDTH * 0.5)


func _clamp_to_field(vec: Vector2) -> Vector2:
	return Vector2(clamp(vec.x, 0.0, FIELD_LENGTH), clamp(vec.y, 0.0, FIELD_WIDTH))


func _event_time(event: Dictionary) -> float:
	if _use_synthetic_timing:
		return float(event.get("__synthetic_time", 0.0))
	var base: Variant = event.get("base", {})
	if base is Dictionary:
		var base_dict: Dictionary = base
		if base_dict.has("t"):
			return float(base_dict.get("t"))
		if base_dict.has("minute"):
			return float(base_dict.get("minute")) * 60.0
	if event.has("timestamp"):
		return float(event.get("timestamp")) / 1000.0
	return 0.0


func _should_use_synthetic_timing(events: Array) -> bool:
	if events.is_empty():
		return false
	for e in events:
		if not (e is Dictionary):
			continue
		var base: Variant = (e as Dictionary).get("base", {})
		if base is Dictionary:
			var base_dict: Dictionary = base
			if float(base_dict.get("t", 0.0)) > 0.0 or float(base_dict.get("minute", 0.0)) > 0.0:
				return false
	return true


func _map_engine_team(team_id_value: Variant) -> int:
	var team_int: int = int(team_id_value)
	if team_int == _home_engine_team_id or team_int == 0:
		return 0
	if team_int == _away_engine_team_id or team_int == 1:
		return 1
	return 1 if team_int > 0 else 0


func _team_color(team_side: int, alpha: float = 1.0) -> Color:
	var base: Color = HOME_COLOR if team_side == 0 else AWAY_COLOR
	return Color(base.r, base.g, base.b, alpha)


func _update_engine_team_ids() -> void:
	if _metadata.has("home_team_id"):
		_home_engine_team_id = int(_metadata.get("home_team_id", 1))
	if _metadata.has("away_team_id"):
		_away_engine_team_id = int(_metadata.get("away_team_id", 2))
	if _home_engine_team_id == _away_engine_team_id:
		_home_engine_team_id = 1
		_away_engine_team_id = 2


func _process_stored_events(stored_events: Array) -> void:
	_pass_lines.clear()
	_shot_points.clear()
	_pressure_points.clear()
	_run_segments.clear()
	_dribble_segments.clear()
	_throughball_lines.clear()

	if stored_events.is_empty():
		return

	for event in stored_events:
		if not (event is Dictionary):
			continue
		var kind: String = str(event.get("type", "")).to_lower()
		match kind:
			"pass":
				if show_pass_lines:
					_handle_pass_event(event, false)
			"shot":
				if show_shot_points:
					_handle_shot_event(event)
			"run":
				if show_run_segments:
					_handle_run_segment(event)
			"dribble":
				if show_dribble_segments:
					_handle_dribble_segment(event)
			"through_ball":
				if show_throughball_lines:
					_handle_throughball_event(event)


func _import_metadata() -> void:
	if _metadata.has("home_name"):
		_home_team_name = str(_metadata.get("home_name", "Home"))
	if _metadata.has("away_name"):
		_away_team_name = str(_metadata.get("away_name", "Away"))
	if _metadata.has("score"):
		var score = _metadata.get("score")
		if score is Dictionary:
			_score_home = int(score.get("home", 0))
			_score_away = int(score.get("away", 0))
	if _metadata.has("hero_id"):
		_hero_player_id = str(_metadata.get("hero_id", ""))
	if _metadata.has("goal_heat_samples"):
		_import_goal_heat_samples(_metadata.get("goal_heat_samples"))


func _import_goal_heat_samples(source: Variant) -> void:
	if source == null or not (source is Array):
		return
	_goal_heat_totals = {0: 0.0, 1: 0.0}
	for entry in source:
		if not (entry is Dictionary):
			continue
		var sample: Dictionary = entry
		var team_side := int(sample.get("team_side", sample.get("goal_side", 0)))
		var x_val := float(sample.get("x", 0.0))
		var y_val := float(sample.get("y", 0.0))
		var weight: float = float(sample.get("weight", sample.get("xg", 0.25)))
		var pos: Vector2 = Vector2(clampf(x_val, 0.0, FIELD_LENGTH), clampf(y_val, 0.0, FIELD_WIDTH))
		_record_goal_heat_sample(team_side, pos, weight)
	_goal_heat_locked = true


func _on_dsa_insight_frame(frame: Dictionary) -> void:
	_dsa_frame = frame.duplicate(true) if frame is Dictionary else {}


func _draw_dsa_overlay() -> void:
	if _dsa_frame.is_empty():
		return

	var lines: Array[String] = []
	var pressure: float = float(_dsa_frame.get("local_pressure", 0.0))
	var gini: float = float(_dsa_frame.get("hub_gini_proxy", 0.0))
	var zone_id: int = int(_dsa_frame.get("ball_zone_id", -1))
	lines.append("DSA p=%.2f g=%.2f z=%d" % [pressure, gini, zone_id])

	var trans: Variant = _dsa_frame.get("zone_transition_last", {})
	if trans is Dictionary and (trans as Dictionary).has("kind"):
		lines.append(
			"T %s %s→%s"
			% [
				str(trans.get("kind", "")),
				str(trans.get("from", "?")),
				str(trans.get("to", "?"))
			]
		)

	var flags: Variant = _dsa_frame.get("qa_flags", [])
	if flags is Array and not flags.is_empty():
		var flag_strs: Array[String] = []
		for f in flags:
			flag_strs.append(str(f))
		lines.append("QA " + ", ".join(flag_strs))

	var font: Font = get_theme_default_font()
	var font_size: int = maxi(get_theme_default_font_size() - 2, 10)
	var pad := Vector2(10, 8)
	var pos := Vector2(16, 16)

	var max_w := 0.0
	var line_h := float(font_size) + 4.0
	for s in lines:
		var sz := font.get_string_size(s, HORIZONTAL_ALIGNMENT_LEFT, -1, font_size)
		max_w = max(max_w, sz.x)

	var rect := Rect2(pos - pad, Vector2(max_w, line_h * lines.size()) + pad * 2.0)
	draw_rect(rect, Color(0.0, 0.0, 0.0, 0.65), true)

	var y := pos.y
	for s in lines:
		draw_string(font, Vector2(pos.x, y + font_size), s, HORIZONTAL_ALIGNMENT_LEFT, -1, font_size, Color(0.95, 0.95, 0.95))
		y += line_h


func _limit_overlay_entries(arr: Array) -> void:
	while arr.size() > MAX_OVERLAY_ENTRIES:
		arr.remove_at(0)
#endregion
