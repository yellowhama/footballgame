extends Control
class_name HorizontalMatchViewer
##
## HorizontalMatchViewer - Socceralia 스타일 가로형 경기 뷰어
##
## TV 중계처럼 가로로 펼쳐진 경기장을 표시합니다.
## 기존 MatchTimelineViewer와 동일한 인터페이스 (apply_position_snapshot)를 제공합니다.
##
## 스펙: docs/spec+@/spec_v4/dev_spec/UI/2025-12-07_SOCCERALIA_HORIZONTAL_VIEW_SPEC.md
##
## Phase20: UnifiedFramePipeline integration (2025-12-18)
## - Connects to UnifiedFramePipeline.snapshot_ready (single source for Session + Timeline)
## - Uses _on_unified_snapshot() for both Session and Timeline modes
## - Events processed via _process_snapshot_events() (unified event handling)
## - Old _check_and_trigger_events() removed (events now in snapshot)
## - See: docs/specs/spec_v5/fix/phase20/PHASE20_UNIFIED_FRAME_PIPELINE_SPEC.md
##

## 의존 스크립트 preload (class_name 인식 문제 해결)
const SoccerPlayerScript := preload("res://scripts/match_pipeline/horizontal/SoccerPlayer.gd")
const TeamColorManagerScript := preload("res://scripts/match_pipeline/horizontal/TeamColorManager.gd")
const NESTextOverlayScript := preload("res://scripts/match_pipeline/horizontal/NESTextOverlay.gd")
const MatchSFXPlayerScript := preload("res://scripts/audio/MatchSFXPlayer.gd")
const NightMatchLightingScript := preload("res://scripts/match_pipeline/horizontal/NightMatchLighting.gd")
const PlayerInfoPanelScript := preload("res://scripts/match_pipeline/horizontal/PlayerInfoPanel.gd")

var SoccerPlayerScene: PackedScene = null

## 오버레이 스타일
enum OverlayStyle { SOCCERALIA, NES }

#region Constants
const METER_TO_PIXEL: float = 10.0
const FIELD_LENGTH_M: float = 105.0
const FIELD_WIDTH_M: float = 68.0
const FIELD_LENGTH_PX: float = FIELD_LENGTH_M * METER_TO_PIXEL  # 1050
const FIELD_WIDTH_PX: float = FIELD_WIDTH_M * METER_TO_PIXEL  # 680

const PLAYER_COUNT: int = 22
const HOME_PLAYER_COUNT: int = 11

## Ball Animation
const BALL_BASE_SCALE: Vector2 = Vector2(1.2, 1.2)  ## 공 기본 크기 (2.0 → 1.2로 축소)
const BALL_BOUNCE_SQUASH: Vector2 = Vector2(1.5, 0.8)  ## 착지 시 납작하게
const BALL_BOUNCE_DURATION: float = 0.12  ## 바운스 복원 시간

## Ball Shadow - P0 Goal Contract 2.5D 표현 (2025-12-12)
## 참조: docs/spec+@/spec_v5/fix/P0_GOAL_CONTRACT.md 섹션 10.6
const SHADOW_BASE_SCALE: Vector2 = Vector2(1.0, 0.5)  ## 그림자 기본 크기 (타원형)
const SHADOW_BASE_ALPHA: float = 0.5  ## 그림자 기본 투명도
const SHADOW_MIN_SCALE: float = 0.3  ## 최소 스케일 (높이 최대일 때)
const SHADOW_MIN_ALPHA: float = 0.15  ## 최소 투명도 (높이 최대일 때)
const SHADOW_HEIGHT_FACTOR: float = 0.2  ## 높이당 스케일/투명도 감소율 (5m에서 완전 감쇠)
const HEIGHT_Y_OFFSET_RATIO: float = 10.0  ## z 1미터 = y -10픽셀 (위로 띄움)
#endregion

#region Export Variables
@export_group("Basic")
@export var auto_connect_controller: bool = true
@export var overlay_style: OverlayStyle = OverlayStyle.SOCCERALIA
@export var controller_id: int = -1

@export_group("Teams")
@export var home_team_id: String = "home"
@export var away_team_id: String = "away"

@export_group("Camera")
@export var enable_camera_follow: bool = true
@export var camera_smooth_speed: float = 5.0

@export_group("Lighting")
@export var night_mode_enabled: bool = false  ## 야간 경기 모드
#endregion

#region Node References
## MatchFieldWorld 씬을 인스턴스로 사용 (MatchFieldWorld.tscn 구조에 맞춤)
@onready var world: Node2D = $SubViewportContainer/SubViewport/MatchFieldWorld
@onready var field_background: Node2D = $SubViewportContainer/SubViewport/MatchFieldWorld/FieldBackground
@onready var field_lines: Node2D = $SubViewportContainer/SubViewport/MatchFieldWorld/FieldLines  ## FieldLineDrawer
@onready var players_container: Node2D = $SubViewportContainer/SubViewport/MatchFieldWorld/PlayersContainer
@onready var ball_sprite: Sprite2D = $SubViewportContainer/SubViewport/MatchFieldWorld/Ball
@onready var camera: Camera2D = $SubViewportContainer/SubViewport/MatchFieldWorld/MatchCamera  ## MatchCamera
## Note: ball_shadow는 MatchFieldWorld에 없으므로 사용하지 않음

## HUD 노드
@onready var score_panel: Control = $HUDLayer/ScorePanel  ## ScorePanel
@onready var transition_hud: Label = $HUDLayer/TransitionHUD  ## Phase23 debug: TransitionSystem
@onready var event_overlay: CanvasLayer = $EventOverlay  ## EventOverlay (Socceralia)
var nes_text_overlay: CanvasLayer = null  ## NESTextOverlay (동적 생성)
var player_info_panel: Control = null  ## PlayerInfoPanel (P2.2: 동적 생성)

## Radial UI (v2.0)
var radial_ui: Control = null  ## RadialDecisionUI (동적 생성)
#endregion

#region State
var _players: Array = []  ## Array[SoccerPlayer]
var _home_players: Array = []  ## Array[SoccerPlayer]
var _away_players: Array = []  ## Array[SoccerPlayer]
var _ball_position: Vector2 = Vector2(525, 340)  ## 센터
var _is_initialized: bool = false

## track_id → player index 매핑 (안정적인 매핑 유지)
var _track_id_to_index: Dictionary = {}
var _next_home_index: int = 0
var _next_away_index: int = 0

## Phase20 Step E: Old event tracking REMOVED (2025-12-18)
## Events now come from UnifiedFramePipeline via snapshot["events"]
## No need for local _events array or tracking indices
# var _events: Array = []  ## DELETED
# var _event_index: int = 0  ## DELETED
# var _last_event_time_ms: int = -1  ## DELETED

## 공 바운스 애니메이션 (2025-12-11)
var _ball_bounce_animating: bool = false
var _prev_ball_height: float = 0.0  ## 이전 프레임 공 높이 (착지 감지용)

## 공 그림자 (2025-12-11 Phase 9)
var _ball_shadow: Sprite2D = null

## SFX 플레이어 (2025-12-11 Phase 9)
var _sfx_player: Node = null  ## MatchSFXPlayer

## Night Match 조명 (2025-12-11 Phase 9)
var _night_lighting: Node2D = null  ## NightMatchLighting

## P2.2: Player Info Panel 상태
var _selected_track_id: int = -1  ## 현재 선택된 선수 track_id
var _last_event_track_id: int = -1  ## 마지막 이벤트 선수 (fallback용)
var _rosters: Dictionary = {}  ## 캐싱된 rosters (overall 조회용)

## Phase 17: Game OS - MatchSetup SSOT
var _match_setup: MatchSetup = null  ## Single source of truth for match rosters

## v2.0: Radial UI 상태
var _current_ball_owner_id: int = -1  ## 현재 공 소유자 track_id
var _controlled_track_id: int = 0  ## 사용자가 조작하는 선수
var _controlled_side: String = "home"  ## 사용자 팀
var _match_id: String = ""  ## 현재 경기 ID

# Match OS Debug Overlay
var _board_overlay: BoardOverlay = null
var _board_overlay_enabled: bool = false
var _team_view_overlay: TeamViewOverlay = null
var _team_view_overlay_enabled: bool = false
var _decision_intent_overlay: DecisionIntentOverlay = null
var _decision_intent_overlay_enabled: bool = false

## Debug modes (2025-12-22 FIX_2512)
enum DebugMode { POSITION_ONLY, EVENTS_ONLY, FULL }  # 순수 position_data만 (이벤트 오버레이 OFF)  # 이벤트만 (위치 무시, 테스트용)  # 정상 모드 (position + events)

var _debug_mode: DebugMode = DebugMode.FULL
#endregion

#region Signals
signal viewer_ready
signal snapshot_applied(t_ms: int)
#endregion


func _ready() -> void:
	_load_scene_resources()
	_setup_field()
	_setup_ball()
	_setup_players()
	_setup_camera()
	_setup_nes_overlay()
	_setup_sfx()
	_setup_night_lighting()
	_setup_player_info_panel()  ## P2.2
	_setup_radial_ui()  ## v2.0

	_is_initialized = true
	viewer_ready.emit()

	if auto_connect_controller:
		_connect_to_timeline_controller()

	# Phase20: Connect to UnifiedFramePipeline (single snapshot emitter)
	_connect_to_pipeline()

	# Setup Match OS BoardOverlay (F3 toggle)
	_setup_board_overlay()
	_setup_team_view_overlay()
	_setup_decision_intent_overlay()


func _load_scene_resources() -> void:
	if ResourceLoader.exists("res://scenes/match_pipeline/horizontal/SoccerPlayer.tscn"):
		SoccerPlayerScene = load("res://scenes/match_pipeline/horizontal/SoccerPlayer.tscn")


func _setup_field() -> void:
	## 필드 라인은 FieldLineDrawer가 자동으로 그림
	if field_lines:
		field_lines.queue_redraw()


func _setup_ball() -> void:
	## 텍스처는 씬에서 이미 설정됨 (MatchFieldWorld.tscn의 Ball 노드)
	if ball_sprite:
		ball_sprite.scale = BALL_BASE_SCALE  ## 2025-12-11: 크기 축소 (2.0 → 1.2)
		ball_sprite.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
		## 텍스처가 없으면 동적 로드
		if ball_sprite.texture == null and ResourceLoader.exists("res://assets/socceralia/ball-idle.png"):
			ball_sprite.texture = load("res://assets/socceralia/ball-idle.png")
		## 볼 초기 위치 설정 (Ball은 world의 직접 자식)
		ball_sprite.position = _ball_position

		## 그림자 스프라이트 생성 (2025-12-11 Phase 9)
		_setup_ball_shadow()


## 공 그림자 생성 (2025-12-11 Phase 9)
func _setup_ball_shadow() -> void:
	if not ball_sprite or not world:
		return

	_ball_shadow = Sprite2D.new()
	_ball_shadow.name = "BallShadow"

	## 그림자 텍스처: 공과 동일한 텍스처를 검은색으로 변조
	if ball_sprite.texture:
		_ball_shadow.texture = ball_sprite.texture

	## 그림자 스타일 설정
	_ball_shadow.scale = SHADOW_BASE_SCALE
	_ball_shadow.modulate = Color(0, 0, 0, SHADOW_BASE_ALPHA)  ## 검은색 반투명
	_ball_shadow.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	_ball_shadow.position = _ball_position

	## 공보다 아래(뒤)에 렌더링되도록 공 앞에 추가
	if ball_sprite and is_instance_valid(ball_sprite):
		var parent = ball_sprite.get_parent()
		if parent and is_instance_valid(parent):
			parent.add_child(_ball_shadow)
			var ball_index = ball_sprite.get_index()
			if ball_index >= 0:
				parent.move_child(_ball_shadow, ball_index)


func _setup_players() -> void:
	## 기존 플레이어 제거
	for child in players_container.get_children():
		child.queue_free()

	_players.clear()
	_home_players.clear()
	_away_players.clear()

	## track_id 매핑 리셋
	_track_id_to_index.clear()
	_next_home_index = 0
	_next_away_index = 0

	## 22명 생성
	for i in range(PLAYER_COUNT):
		var player: Node2D  ## SoccerPlayer
		if SoccerPlayerScene:
			player = SoccerPlayerScene.instantiate()
		else:
			player = SoccerPlayerScript.new()
			_setup_player_nodes(player)

		var is_home := i < HOME_PLAYER_COUNT
		player.team_id = 0 if is_home else 1
		player.jersey_number = (i % 11) + 1

		## Phase 5: Mark goalkeepers (2025-12-22 FIX_2512)
		## track_id 0, 11 = GK
		if i == 0 or i == 11:
			# SoccerPlayer is a Node2D; do not use Dictionary-style `has()`.
			# Mark GK via SoccerPlayer API (sets PotagonSprite.is_goalkeeper).
			if player.has_method("set_role"):
				player.set_role("GK")
			player.z_index = 5  # Above field players but below ball
			player.modulate = Color(1.0, 1.0, 0.8)  # Slight yellow tint for visibility

		## 기본 헤어 스타일 (레거시 - Potagon은 불필요)
		# var position_name := _get_default_position(i % 11)
		# player.hair_style = TeamColorManagerScript.get_hair_style_for_player(str(i), position_name)

		players_container.add_child(player)
		_players.append(player)

		if is_home:
			_home_players.append(player)
		else:
			_away_players.append(player)

	## 팀 컬러 적용
	TeamColorManagerScript.setup_home_away_teams(_home_players, _away_players)


func _setup_player_nodes(player: Node2D) -> void:  ## SoccerPlayer
	## SoccerPlayer에 필요한 자식 노드 생성 (씬 없이 사용 시)
	var sprite := Sprite2D.new()
	sprite.name = "Sprite2D"
	player.add_child(sprite)

	var shadow := Sprite2D.new()
	shadow.name = "Shadow"
	player.add_child(shadow)

	var label := Label.new()
	label.name = "NumberLabel"
	player.add_child(label)


func _setup_camera() -> void:
	if camera:
		camera.smoothing_speed = camera_smooth_speed
		if enable_camera_follow:
			camera.set_follow_mode()
		else:
			camera.set_full_pitch_view()

		## 공 스프라이트를 타겟으로 설정
		camera.target_node = ball_sprite


func _setup_nes_overlay() -> void:
	## NES 스타일 오버레이가 필요하면 동적 생성
	if overlay_style == OverlayStyle.NES:
		nes_text_overlay = NESTextOverlayScript.new()
		nes_text_overlay.name = "NESTextOverlay"
		add_child(nes_text_overlay)


## SFX 플레이어 설정 (2025-12-11 Phase 9)
func _setup_sfx() -> void:
	_sfx_player = MatchSFXPlayerScript.new()
	_sfx_player.name = "MatchSFXPlayer"
	add_child(_sfx_player)
	print("[HorizontalMatchViewer] MatchSFXPlayer initialized")


## Night Match 조명 설정 (2025-12-11 Phase 9)
func _setup_night_lighting() -> void:
	_night_lighting = NightMatchLightingScript.new()
	_night_lighting.name = "NightMatchLighting"

	## world(MatchFieldWorld) 안에 추가해야 조명이 적용됨
	if world:
		world.add_child(_night_lighting)
		_night_lighting.set_ball_target(ball_sprite)

		## export 변수로 야간 모드 활성화 설정
		if night_mode_enabled:
			_night_lighting.set_enabled(true)

		print("[HorizontalMatchViewer] NightMatchLighting initialized (enabled=%s)" % night_mode_enabled)


## P2.2: Player Info Panel 설정
func _setup_player_info_panel() -> void:
	if not has_node("HUDLayer"):
		push_warning("[HorizontalMatchViewer] HUDLayer not found, cannot setup PlayerInfoPanel")
		return

	player_info_panel = PlayerInfoPanelScript.new()
	player_info_panel.name = "PlayerInfoPanel"
	$HUDLayer.add_child(player_info_panel)
	player_info_panel.visible = false
	print("[HorizontalMatchViewer] PlayerInfoPanel initialized")


## v2.0: Radial UI 설정
func _setup_radial_ui() -> void:
	if not has_node("HUDLayer"):
		push_warning("[HorizontalMatchViewer] HUDLayer not found, cannot setup RadialUI")
		return

	# Load RadialDecisionUI script
	var RadialUIScript = load("res://scripts/ui/RadialDecisionUI.gd")
	if not RadialUIScript:
		push_error("[HorizontalMatchViewer] Failed to load RadialDecisionUI.gd")
		return

	radial_ui = RadialUIScript.new()
	radial_ui.name = "RadialDecisionUI"
	radial_ui.set_anchors_preset(Control.PRESET_FULL_RECT)
	radial_ui.controlled_track_id = _controlled_track_id
	radial_ui.controlled_side = _controlled_side
	radial_ui.controller_id = controller_id
	radial_ui.match_id = _match_id
	$HUDLayer.add_child(radial_ui)
	radial_ui.visible = true  # Always visible (hides internally)
	print("[HorizontalMatchViewer] RadialDecisionUI initialized")


func _get_default_position(index: int) -> String:
	const POSITIONS := ["GK", "LB", "CB", "CB", "RB", "DM", "CM", "CM", "LW", "ST", "RW"]
	return POSITIONS[index] if index < POSITIONS.size() else "CM"


#region Public API (Viewer compatibility)


## Phase20 Step E: set_events() DEPRECATED (2025-12-18)
## Events are now handled by UnifiedFramePipeline, not locally stored
## Kept for backwards compatibility - does nothing
func set_events(events: Array) -> void:
	# NO-OP: Events come from UnifiedFramePipeline via snapshot["events"]
	if OS.is_debug_build():
		print("[HorizontalMatchViewer] set_events() called but is deprecated (Phase20)")


## 야간 경기 모드 설정 (2025-12-11 Phase 9)
func set_night_mode(enabled: bool) -> void:
	night_mode_enabled = enabled
	if _night_lighting and _night_lighting.has_method("set_enabled"):
		_night_lighting.set_enabled(enabled)


## 야간 경기 모드 토글
func toggle_night_mode() -> bool:
	set_night_mode(not night_mode_enabled)
	return night_mode_enabled


## 야간 어둠 정도 조절 (0.0 = 완전 어둠, 1.0 = 낮)
func set_darkness_level(level: float) -> void:
	if _night_lighting and _night_lighting.has_method("set_darkness_level"):
		_night_lighting.set_darkness_level(level)


## 포지션 스냅샷 적용 (메인 인터페이스)
func apply_position_snapshot(snapshot: Dictionary) -> void:
	if not _is_initialized:
		return

	var t_ms: int = snapshot.get("t_ms", 0)

	## 시간 업데이트
	set_match_time(t_ms)

	## Phase20 Step E: Event handling moved to _on_unified_snapshot()
	## Events are now in snapshot["events"] and processed by _process_snapshot_events()
	## Old _check_and_trigger_events() removed - events come from UnifiedFramePipeline

	## 공 위치 업데이트
	var ball_data: Dictionary = snapshot.get("ball", {})
	if ball_data.has("pos"):
		var ball_pos: Vector2 = ball_data.get("pos", Vector2.ZERO)
		_update_ball_position(ball_pos, ball_data.get("z", 0.0))

	## 선수 위치 업데이트
	var players_data: Dictionary = snapshot.get("players", {})
	_update_players_from_snapshot(players_data, t_ms)

	## 스코어 업데이트 (있으면)
	var score: Dictionary = snapshot.get("score", {})
	if not score.is_empty():
		_on_score_updated(score)

	## Phase23: TransitionSystem debug HUD (optional)
	_update_transition_hud(snapshot)

	snapshot_applied.emit(t_ms)

	## P2.2: PlayerInfoPanel 업데이트
	_update_player_info_panel(snapshot)

	## v2.0: RadialUI 업데이트
	_update_radial_ui(snapshot)

	## Match OS: BoardOverlay 업데이트
	if _board_overlay and _board_overlay_enabled and snapshot.has("field_board"):
		_board_overlay.current_snapshot = snapshot["field_board"]
		_board_overlay.queue_redraw()


func _update_transition_hud(snapshot: Dictionary) -> void:
	if not transition_hud:
		return

	var remaining_ms := int(snapshot.get("transition_remaining_ms", -1))
	if remaining_ms <= 0:
		transition_hud.visible = false
		return

	transition_hud.visible = true
	transition_hud.text = "TRANSITION: %dms" % remaining_ms


## Phase20 Step E: _check_and_trigger_events() DELETED
## Event handling now unified in _on_unified_snapshot() via _process_snapshot_events()
## Events come from UnifiedFramePipeline for both Session and Timeline modes


func _update_ball_position(engine_pos: Vector2, height: float = 0.0) -> void:
	## P0 Goal Contract 2.5D 표현 (2025-12-12)
	## 참조: docs/spec+@/spec_v5/fix/P0_GOAL_CONTRACT.md 섹션 10.6
	##
	## 핵심 원칙:
	## - 그림자(Shadow)는 바닥(물리 좌표)에 고정
	## - 공(Body)은 높이에 따라 위로 이동 (Y 오프셋)
	## - 공 크기는 변하지 않음 (원근감 X)

	var screen_pos := _engine_to_screen(engine_pos)
	_ball_position = screen_pos  ## 물리적 위치 (그림자 기준)

	## 높이에 따른 Y 오프셋 계산 (위로 띄움)
	var height_offset := height * HEIGHT_Y_OFFSET_RATIO

	if ball_sprite:
		## 공은 물리 위치에서 높이만큼 위로 이동
		ball_sprite.position = Vector2(screen_pos.x, screen_pos.y - height_offset)

		## 착지 감지 - 공중에서 땅으로 내려올 때 바운스 (2025-12-11)
		if _prev_ball_height > 0.1 and height < 0.1:
			_play_ball_bounce()

		## 바운스 애니메이션 중이 아닐 때만 기본 스케일 유지
		## P0: 공 크기는 변하지 않음 (원근감 없음)
		if not _ball_bounce_animating:
			ball_sprite.scale = BALL_BASE_SCALE

		## Z-Index: 높이 뜨면 선수 위로 (P0 스펙)
		if height > 2.0:
			ball_sprite.z_index = 10  ## 선수 머리 위
		else:
			ball_sprite.z_index = 0  ## Y-Sort 적용

		_prev_ball_height = height

	## 그림자 업데이트 - 바닥 고정 (P0 스펙)
	_update_ball_shadow(screen_pos, height)

	## 카메라 업데이트 - 물리 위치 기준 (공이 아닌 그림자 따라감)
	if camera:
		camera.update_ball_position(screen_pos)


## 공 그림자 업데이트 (2025-12-11 Phase 9)
## 높이에 따라 그림자 크기와 투명도 조정
func _update_ball_shadow(screen_pos: Vector2, height: float) -> void:
	if not _ball_shadow:
		return

	## 그림자는 항상 땅에 있음 (공 아래 고정)
	_ball_shadow.position = screen_pos

	## 높이에 따른 스케일 조정 (높을수록 작아짐)
	var scale_factor := 1.0 - (height * SHADOW_HEIGHT_FACTOR)
	scale_factor = clamp(scale_factor, SHADOW_MIN_SCALE, 1.0)
	_ball_shadow.scale = SHADOW_BASE_SCALE * scale_factor

	## 높이에 따른 투명도 조정 (높을수록 흐려짐)
	var alpha := SHADOW_BASE_ALPHA - (height * SHADOW_HEIGHT_FACTOR)
	alpha = clamp(alpha, SHADOW_MIN_ALPHA, SHADOW_BASE_ALPHA)
	_ball_shadow.modulate.a = alpha


## 공 바운스 애니메이션 (2025-12-11)
## 착지 시 squash/stretch 효과
func _play_ball_bounce() -> void:
	if _ball_bounce_animating or not ball_sprite:
		return
	_ball_bounce_animating = true

	## SFX 재생 (2025-12-11 Phase 9)
	if _sfx_player and _sfx_player.has_method("play_ball_bounce"):
		_sfx_player.play_ball_bounce()

	## 납작하게 → 원래 크기로 복원
	var tween := create_tween()
	tween.tween_property(ball_sprite, "scale", BALL_BOUNCE_SQUASH, 0.03)  ## 빠르게 납작
	tween.tween_property(ball_sprite, "scale", BALL_BASE_SCALE, BALL_BOUNCE_DURATION)  ## 복원
	tween.tween_callback(_on_ball_bounce_finished)


func _on_ball_bounce_finished() -> void:
	_ball_bounce_animating = false


## Process events from snapshot (session or timeline mode)
## Phase X: Session Camera Effects (2025-12-18)
## - Unified event processing for both session and timeline modes
## - Events from snapshot.get("events", []) trigger camera effects
func _process_snapshot_events(t_ms: int, events: Array) -> void:
	# Skip events in POSITION_ONLY mode (2025-12-22 FIX_2512)
	if _debug_mode == DebugMode.POSITION_ONLY:
		return

	if events.is_empty():
		return

	for event in events:
		# Trigger EventOverlay
		if event_overlay and event_overlay.has_method("trigger_action_from_event"):
			event_overlay.trigger_action_from_event(event)

		# Trigger camera effects
		_trigger_camera_effect_for_event(event)

	if OS.is_debug_build():
		print("[HorizontalMatchViewer] Processed %d events at t_ms=%d" % [events.size(), t_ms])


## 이벤트에 따른 카메라 효과 트리거 (2025-12-11 Phase 9)
func _trigger_camera_effect_for_event(event: Dictionary) -> void:
	var event_type: String = (
		str(event.get(MatchEventKeys.TYPE, event.get("type", event.get("kind", "")))).strip_edges().to_lower()
	)

	## SFX 재생 (2025-12-11 Phase 9)
	if _sfx_player and _sfx_player.has_method("play_for_event"):
		_sfx_player.play_for_event(event_type)

	if not camera:
		return

	var pos_data = event.get("pos", null)
	var target_pos: Vector2 = _ball_position  ## 기본값: 현재 공 위치 (screen)
	var has_valid_target_pos := false
	var engine_pos: Vector2 = Vector2.ZERO

	## 이벤트 위치 파싱
	if pos_data is Vector2:
		engine_pos = pos_data
		has_valid_target_pos = true
	elif pos_data is Dictionary:
		engine_pos = Vector2(float(pos_data.get("x", 0.0)), float(pos_data.get("y", 0.0)))
		has_valid_target_pos = true
	elif pos_data is Array and pos_data.size() >= 2:
		engine_pos = Vector2(float(pos_data[0]), float(pos_data[1]))
		has_valid_target_pos = true

	# Guard: ignore invalid/sentinel positions to prevent camera snap-to-corner.
	if has_valid_target_pos:
		if engine_pos == Vector2.ZERO:
			has_valid_target_pos = false
		elif engine_pos.x < 0.0 or engine_pos.x > FIELD_LENGTH_M or engine_pos.y < 0.0 or engine_pos.y > FIELD_WIDTH_M:
			has_valid_target_pos = false
		else:
			target_pos = _engine_to_screen(engine_pos)

	## 이벤트 타입에 따른 카메라 효과
	match event_type:
		"goal":
			if has_valid_target_pos and camera.has_method("zoom_in_for_goal"):
				camera.zoom_in_for_goal(target_pos)
		"shot", "shot_on_target", "shot_off_target":
			if has_valid_target_pos and camera.has_method("zoom_in_for_shot"):
				camera.zoom_in_for_shot(target_pos)
		"foul", "yellow_card", "red_card":
			if has_valid_target_pos and camera.has_method("zoom_in_for_foul"):
				camera.zoom_in_for_foul(target_pos)
		"save":
			## 골키퍼 세이브: 짧은 줌인
			if has_valid_target_pos and camera.has_method("zoom_in_for_shot"):
				camera.zoom_in_for_shot(target_pos, 2.3, 0.5)


## 디버그: 첫 호출 시만 로깅
var _players_debug_logged: bool = false


func _update_players_from_snapshot(players_data: Dictionary, t_ms: int) -> void:
	## 디버그: 첫 호출 시 데이터 구조 로깅
	if not _players_debug_logged and not players_data.is_empty():
		_players_debug_logged = true
		print("[HorizontalMatchViewer] _update_players_from_snapshot called with %d players" % players_data.size())
		var sample_count := 0
		for pid in players_data:
			if sample_count < 3:
				var d: Dictionary = players_data[pid]
				print(
					(
						"  player_id=%s, pos=%s, team_id=%s"
						% [str(pid), str(d.get("pos", "?")), str(d.get("team_id", "?"))]
					)
				)
				sample_count += 1
		print("  _players array size: %d" % _players.size())

	var successful_updates := 0
	var failed := 0

	# ✅ SSOT: snapshot.players keys must be track_id (0..21). We index directly.
	for key in players_data.keys():
		var data: Dictionary = players_data[key]
		var pos: Vector2 = data.get("pos", Vector2.ZERO)
		var action: String = data.get("action", "idle")
		var velocity_engine: Vector2 = data.get("velocity", Vector2.ZERO)
		var velocity: Vector2 = _engine_to_screen_velocity(velocity_engine)

		var track_id := -1
		if key is int:
			track_id = key
		else:
			var ks := str(key)
			if ks.is_valid_int():
				track_id = int(ks)

		if track_id >= 0 and track_id < _players.size():
			_players[track_id].update_from_snapshot(t_ms, pos, action, velocity)
			successful_updates += 1
		else:
			failed += 1
			if failed <= 3:  # 처음 3개만 로그
				var legacy_idx := _get_player_index(key)
				print(
					(
						"[HorizontalMatchViewer] BAD snapshot player key (expect track_id 0..21): %s (legacy_idx=%d)"
						% [str(key), legacy_idx]
					)
				)

	if failed > 0:
		print(
			(
				"[HorizontalMatchViewer] Player update: %d ok, %d failed (expect 22 ok when SSOT is track_id)"
				% [successful_updates, failed]
			)
		)


func _get_player_index(player_id) -> int:
	## 2025-12-17: player_id는 실제 선수 ID (from roster)
	## PositionSnapshotAdapter가 이미 player metadata를 매칭했음
	## _players 배열에서 매칭되는 선수를 찾아야 함

	# Fast-path: track_id 인덱스 (0-21)
	if player_id is int and player_id >= 0 and player_id < _players.size():
		return player_id

	var pid_str := str(player_id)

	# Primary: roster player id (TeamColorManager stores it in SoccerPlayer.player_id)
	for i in range(_players.size()):
		var player = _players[i]
		if player is SoccerPlayer and str(player.player_id) == pid_str:
			return i

	## Fallback: track_id로 매칭 (레거시 지원)
	## track_id는 0-21 순서 인덱스 (home=0-10, away=11-21)
	if player_id is int:
		# 만약 player_id가 0-21 범위면 track_id로 간주
		if player_id >= 0 and player_id < _players.size():
			return player_id
	elif player_id is String:
		## "home_0", "away_0" 형식 지원 (레거시)
		if player_id.begins_with("home_"):
			return int(player_id.substr(5))
		elif player_id.begins_with("away_"):
			return HOME_PLAYER_COUNT + int(player_id.substr(5))
		## 숫자 문자열 ("0", "15" 등)
		elif player_id.is_valid_int():
			var idx := int(player_id)
			if idx >= 0 and idx < _players.size():
				return idx

	return -1


func _engine_to_screen(engine_pos: Vector2) -> Vector2:
	## Rust 엔진 좌표계 (MeterPos - 2025-12-22 FIX_2512 수정):
	##   X: 0-105m (LENGTH, 골라인 방향, home goal → away goal)
	##   Y: 0-68m (WIDTH, 터치라인 방향, touchline → touchline)
	##
	## 가로 뷰어 화면 좌표계:
	##   X: 0-1050px (LENGTH, 골라인 방향)
	##   Y: 0-680px (WIDTH, 터치라인 방향)
	##
	## 변환: X → X (동일 비율 10.0), Y → Y (동일 비율 10.0)
	const ENGINE_FIELD_LENGTH: float = 105.0  # X축 (meters)
	const ENGINE_FIELD_WIDTH: float = 68.0  # Y축 (meters)
	const SCREEN_FIELD_LENGTH: float = 1050.0  # X축 (pixels)
	const SCREEN_FIELD_WIDTH: float = 680.0  # Y축 (pixels)

	const SCALE_FACTOR: float = 10.0  # 10 pixels per meter (both axes)

	return Vector2(engine_pos.x * SCALE_FACTOR, engine_pos.y * SCALE_FACTOR)  # X: 0-105m → 0-1050px (10x)  # Y: 0-68m → 0-680px (10x)


func _engine_to_screen_velocity(engine_vel: Vector2) -> Vector2:
	## 속도 벡터 변환 (위치와 동일한 스케일 - 2025-12-22 FIX_2512 수정)
	const SCALE_FACTOR: float = 10.0  # 10 pixels per meter (both axes)

	return Vector2(engine_vel.x * SCALE_FACTOR, engine_vel.y * SCALE_FACTOR)  # vx → vx (same scaling)  # vy → vy (same scaling)


func _on_score_updated(score: Dictionary) -> void:
	## 스코어 패널 업데이트
	if score_panel and score_panel.has_method("set_score"):
		var home_score: int = score.get("home", 0)
		var away_score: int = score.get("away", 0)
		score_panel.set_score(home_score, away_score)


## P2.2: PlayerInfoPanel 업데이트 로직
func _update_player_info_panel(snapshot: Dictionary) -> void:
	if not player_info_panel:
		return

	var selected_track_id: int = -1

	## 1. Check ball owner
	var ball_data: Dictionary = snapshot.get("ball", {})
	var owner_id = ball_data.get("owner_id", -1)
	if owner_id >= 0 and owner_id < 22:
		selected_track_id = owner_id

	## 2. Check recent events
	if selected_track_id < 0:
		var events: Array = snapshot.get("events", [])
		if not events.is_empty():
			for i in range(events.size() - 1, -1, -1):
				var ev: Dictionary = events[i]
				var player_track_id := _extract_track_id_from_event(ev)
				if player_track_id >= 0:
					selected_track_id = player_track_id
					_last_event_track_id = player_track_id
					break

	## 3. Fallback to last known
	if selected_track_id < 0 and _last_event_track_id >= 0:
		selected_track_id = _last_event_track_id

	## Update panel
	if selected_track_id != _selected_track_id:
		_selected_track_id = selected_track_id
		if selected_track_id >= 0:
			var players_data: Dictionary = snapshot.get("players", {})
			var player_key: String = str(selected_track_id)
			if players_data.has(player_key):
				var player_data: Dictionary = players_data[player_key]
				player_data["track_id"] = selected_track_id  ## Ensure track_id is in data
				player_info_panel.update_player_from_snapshot(player_data)
			else:
				player_info_panel.clear()
		else:
			player_info_panel.clear()
	elif selected_track_id >= 0:
		## Update stamina even if selection unchanged
		var players_data: Dictionary = snapshot.get("players", {})
		var player_key: String = str(selected_track_id)
		if players_data.has(player_key):
			var player_data: Dictionary = players_data[player_key]
			player_data["track_id"] = selected_track_id
			player_info_panel.update_player_from_snapshot(player_data)


## v2.0: Radial UI 업데이트 로직
func _update_radial_ui(snapshot: Dictionary) -> void:
	if not radial_ui:
		return

	# Get ball owner from snapshot
	var ball_data: Dictionary = snapshot.get("ball", {})
	var new_owner_id: int = ball_data.get("owner_id", -1)

	# Track ball owner changes
	if new_owner_id != _current_ball_owner_id:
		_current_ball_owner_id = new_owner_id
		_on_ball_owner_changed(new_owner_id, snapshot)


## v2.0: Ball owner change handler
func _on_ball_owner_changed(owner_track_id: int, snapshot: Dictionary) -> void:
	if not radial_ui:
		return

	if owner_track_id == _controlled_track_id:
		# Controlled player got the ball - show UI
		var player_screen_pos := _get_player_screen_pos(owner_track_id)
		# Use public API instead of private members
		if radial_ui.has_method("set_center"):
			radial_ui.set_center(player_screen_pos)
			radial_ui.show()
		elif "position" in radial_ui:
			radial_ui.position = player_screen_pos
			radial_ui.show()
		print("[HorizontalMatchViewer] Ball gained by controlled player - UI shown at %s" % player_screen_pos)
	else:
		# Ball lost or possessed by opponent - hide UI
		if radial_ui.visible:
			radial_ui.hide()
			print("[HorizontalMatchViewer] Ball lost - UI hidden")


## v2.0: Get player screen position
func _get_player_screen_pos(track_id: int) -> Vector2:
	if _track_id_to_index.has(track_id):
		var player_index: int = _track_id_to_index[track_id]
		if player_index < _players.size():
			var player = _players[player_index]
			if player:
				return player.global_position

	# Fallback to center
	return get_viewport_rect().size / 2.0


## P2.2: 이벤트에서 track_id 추출
func _extract_track_id_from_event(event: Dictionary) -> int:
	# Event SSOT: prefer player_track_id (0..21)
	var tid: int = int(event.get(MatchEventKeys.PLAYER_TRACK_ID, -1))
	if tid >= 0 and tid < 22:
		return tid

	var player_id = event.get("player_id", event.get("from_player_id", event.get("actor_id", -1)))

	if player_id is int and player_id >= 0 and player_id < 22:
		return player_id
	elif player_id is String:
		if player_id.begins_with("home_"):
			var idx: int = int(player_id.substr(5))
			if idx >= 0 and idx < 11:
				return idx
		elif player_id.begins_with("away_"):
			var idx: int = int(player_id.substr(5))
			if idx >= 0 and idx < 11:
				return 11 + idx
		elif player_id.is_valid_int():
			var idx: int = int(player_id)
			if idx >= 0 and idx < 22:
				return idx

	return -1


#endregion

#region Team Setup


## Phase 17: Setup teams from MatchSetup SSOT (NEW - Game OS mode)
## Replaces setup_teams() for OS-compliant workflow
func setup_teams_from_match_setup(match_setup: MatchSetup) -> void:
	if not match_setup:
		push_error("[HorizontalMatchViewer] MatchSetup is null")
		return

	print("[HorizontalMatchViewer] Setting up teams from MatchSetup (Game OS mode)")

	# Store MatchSetup reference
	_match_setup = match_setup

	# Extract rosters from MatchSetup
	var home_roster = []
	var away_roster = []

	# Build home roster from MatchSetup
	for i in range(11):
		var player = match_setup.get_player(i)
		if player:
			home_roster.append(
				{
					"id": player.uid,
					"name": player.name,
					"position": player.position,
					"jersey_number": player.jersey_number,
					"overall": player.overall
				}
			)

	# Build away roster from MatchSetup
	for i in range(11, 22):
		var player = match_setup.get_player(i)
		if player:
			away_roster.append(
				{
					"id": player.uid,
					"name": player.name,
					"position": player.position,
					"jersey_number": player.jersey_number,
					"overall": player.overall
				}
			)

	# Cache rosters for PlayerInfoPanel
	_rosters = {"home": home_roster, "away": away_roster}
	if player_info_panel and player_info_panel.has_method("set_rosters"):
		player_info_panel.set_rosters(_rosters)

	# Set team IDs
	home_team_id = match_setup.home_team.side if match_setup.home_team else "home"
	away_team_id = match_setup.away_team.side if match_setup.away_team else "away"

	# Setup players using TeamColorManager
	TeamColorManagerScript.setup_team_players(_home_players, home_team_id, home_roster)
	TeamColorManagerScript.setup_team_players(_away_players, away_team_id, away_roster)

	# Set track_id and side for each player
	for i in range(_home_players.size()):
		var player = _home_players[i]
		player.track_id = i
		player.side = "home"

	for i in range(_away_players.size()):
		var player = _away_players[i]
		player.track_id = HOME_PLAYER_COUNT + i
		player.side = "away"

	print("[HorizontalMatchViewer] ✅ Teams setup complete from MatchSetup")


## 팀 정보로 선수 초기화 (Legacy - for backward compatibility)
## NOTE: Prefer setup_teams_from_match_setup() for Game OS workflow
func setup_teams(home_roster: Array, away_roster: Array, home_id: String = "home", away_id: String = "away") -> void:
	home_team_id = home_id
	away_team_id = away_id

	## P2.2: rosters 캐싱 (PlayerInfoPanel용)
	_rosters = {"home": home_roster, "away": away_roster}
	if player_info_panel and player_info_panel.has_method("set_rosters"):
		player_info_panel.set_rosters(_rosters)

	TeamColorManagerScript.setup_team_players(_home_players, home_team_id, home_roster)
	TeamColorManagerScript.setup_team_players(_away_players, away_team_id, away_roster)

	## v2.0: Set track_id and side for each player
	for i in range(_home_players.size()):
		var player = _home_players[i]
		player.track_id = i
		player.side = "home"

	for i in range(_away_players.size()):
		var player = _away_players[i]
		player.track_id = HOME_PLAYER_COUNT + i
		player.side = "away"


## Phase 17: Get player name from track_id using MatchSetup SSOT
func get_player_name_from_match_setup(track_id: int) -> String:
	if _match_setup:
		return _match_setup.get_player_name(track_id)
	return "Unknown"


## Phase 17: Get player info from track_id using MatchSetup SSOT
func get_player_info_from_match_setup(track_id: int) -> Dictionary:
	if _match_setup:
		var player = _match_setup.get_player(track_id)
		if player:
			return {
				"uid": player.uid,
				"name": player.name,
				"position": player.position,
				"jersey_number": player.jersey_number,
				"overall": player.overall,
				"technical": player.technical,
				"mental": player.mental,
				"physical": player.physical
			}
	return {}


## 팀 색상만 변경
func set_team_colors(home_id: String, away_id: String) -> void:
	home_team_id = home_id
	away_team_id = away_id

	for player in _home_players:
		TeamColorManagerScript.apply_team_color_to_player(player, home_team_id)

	for player in _away_players:
		TeamColorManagerScript.apply_team_color_to_player(player, away_team_id)


## 커스텀 팀 유니폼으로 팀 설정 (MyTeamData 연동)
## home_roster/away_roster: [{ "id": "p1", "position": "ST", "jersey_number": 9, "appearance": { "hair_folder": "black" } }, ...]
## home_uniform/away_uniform: { "primary": "#FF0000", "secondary": "#FFFFFF", "pattern_type": 0 }
func setup_teams_with_uniform(
	home_roster: Array, away_roster: Array, home_uniform: Dictionary, away_uniform: Dictionary
) -> void:
	TeamColorManagerScript.setup_team_with_appearance(_home_players, home_roster, home_uniform)
	TeamColorManagerScript.setup_team_with_appearance(_away_players, away_roster, away_uniform)


## MyTeamData와 연동하여 홈팀을 마이팀으로 설정
func setup_my_team_as_home(
	my_team_data: Node, my_roster: Array, opponent_roster: Array, opponent_id: String = "away"
) -> void:
	if not my_team_data:
		push_warning("[HorizontalMatchViewer] MyTeamData not provided")
		return

	## 마이팀 유니폼 가져오기
	var my_uniform: Dictionary = {}
	if my_team_data.has_method("get_team_uniform"):
		my_uniform = my_team_data.get_team_uniform(true)  ## 홈 유니폼

	## 마이팀이 홈
	TeamColorManagerScript.setup_team_with_appearance(_home_players, my_roster, my_uniform)

	## 상대팀
	var opponent_colors: Dictionary = TeamColorManagerScript.get_team_colors(opponent_id)
	var opponent_uniform := {
		"primary": "#" + opponent_colors.get("primary", Color.BLUE).to_html(false),
		"secondary": "#" + opponent_colors.get("secondary", Color.WHITE).to_html(false),
		"pattern_type": opponent_colors.get("pattern", 0)
	}
	TeamColorManagerScript.setup_team_with_appearance(_away_players, opponent_roster, opponent_uniform)


#endregion

#region Timeline Controller Integration


func _connect_to_timeline_controller() -> void:
	## Phase20: Pipeline handles snapshot emission for both Session and Timeline
	## Timeline controller only drives the playhead - no direct connection needed
	## (Keep function for compatibility)

	## Phase 9: Connect to MatchSimulationManager for session matches
	_connect_to_match_simulation_manager()


func _connect_to_match_simulation_manager() -> void:
	## Phase20: Connect to pipeline for snapshots, NOT to MatchSimulationManager
	## Still connect to MatchSimulationManager for HUD state updates (time/score)
	var manager: Node = null

	if Engine.has_singleton("MatchSimulationManager"):
		manager = Engine.get_singleton("MatchSimulationManager")
	elif has_node("/root/MatchSimulationManager"):
		manager = get_node("/root/MatchSimulationManager")

	if manager:
		# Phase20: Snapshot emission comes from UnifiedFramePipeline (no manager snapshot wiring)
		# (legacy manager snapshot wiring removed)

		# Keep match_state_updated for HUD (time, score, period)
		if manager.has_signal("match_state_updated"):
			if not manager.match_state_updated.is_connected(_on_match_state_updated):
				manager.match_state_updated.connect(_on_match_state_updated)
				print("[HorizontalMatchViewer] Connected to MatchSimulationManager.match_state_updated")
			else:
				print("[HorizontalMatchViewer] Already connected to MatchSimulationManager.match_state_updated")
		else:
			push_warning("[HorizontalMatchViewer] MatchSimulationManager has no match_state_updated signal")
	else:
		# Not a warning - this is normal for timeline-only mode
		if OS.is_debug_build():
			print("[HorizontalMatchViewer] MatchSimulationManager not found - viewer in timeline-only mode")


## Phase20: Connect to UnifiedFramePipeline (single snapshot emitter for Session + Timeline)
func _connect_to_pipeline() -> void:
	if not has_node("/root/UnifiedFramePipeline"):
		push_error("[HorizontalMatchViewer] UnifiedFramePipeline not found!")
		return

	var pipeline = get_node("/root/UnifiedFramePipeline")
	if not pipeline.snapshot_ready.is_connected(_on_unified_snapshot):
		pipeline.snapshot_ready.connect(_on_unified_snapshot)

		if OS.is_debug_build():
			print("[HorizontalMatchViewer] Connected to UnifiedFramePipeline.snapshot_ready")


## Phase20: Unified snapshot handler (Session + Timeline)
func _on_unified_snapshot(t_ms: int, snapshot: Dictionary) -> void:
	if OS.is_debug_build():
		var event_count = snapshot.get("events", []).size()
		print("[HorizontalMatchViewer] Unified snapshot: t=%dms, events=%d" % [t_ms, event_count])

	apply_position_snapshot(snapshot)

	# Process events (same logic as before)
	var events = snapshot.get("events", [])
	if not events.is_empty():
		_process_snapshot_events(t_ms, events)


## Phase20: Legacy snapshot handlers removed - replaced by _on_unified_snapshot()


func _on_match_state_updated(state: Dictionary) -> void:
	## Match state 업데이트 (점수, 시간 등)
	var time_ms = state.get("time_ms", 0)
	var score = state.get("score", {"home": 0, "away": 0})

	if OS.is_debug_build():
		var time_min = time_ms / 60000
		print("[HorizontalMatchViewer] Match state: %d min, %d-%d" % [time_min, score["home"], score["away"]])

	## Phase 9.2: Update HUD with match state
	# Update scoreboard
	if score_panel and score_panel.has_method("set_score"):
		score_panel.set_score(score.get("home", 0), score.get("away", 0))

	# Update match time
	if score_panel and score_panel.has_method("set_time_ms"):
		score_panel.set_time_ms(time_ms)

	# TODO: Update possession stats (if available in state)
	# var possession = state.get("possession", "")
	# if possession != "":
	#     _update_possession_indicator(possession)


#endregion

#region Camera Control


func set_camera_mode_follow() -> void:
	if camera:
		camera.set_follow_mode()


func set_camera_mode_full() -> void:
	if camera:
		camera.set_full_pitch_view()


func set_camera_mode_tactical(attacking_right: bool = true) -> void:
	if camera:
		camera.set_tactical_half_view(attacking_right)


func trigger_goal_effect() -> void:
	if camera:
		camera.camera_shake(8.0, 0.5)
		camera.highlight_goal_moment(2.0)


#endregion

#region HUD Control


## 골 오버레이 표시
func show_goal_overlay(team_name: String = "", scorer_name: String = "") -> void:
	if overlay_style == OverlayStyle.NES and nes_text_overlay:
		if nes_text_overlay.has_method("show_goal"):
			nes_text_overlay.show_goal()
	elif event_overlay and event_overlay.has_method("show_goal"):
		event_overlay.show_goal(team_name, scorer_name)
	trigger_goal_effect()


## 하프타임 오버레이 표시
func show_halftime_overlay() -> void:
	if overlay_style == OverlayStyle.NES and nes_text_overlay:
		if nes_text_overlay.has_method("show_half_time"):
			nes_text_overlay.show_half_time()
	elif event_overlay and event_overlay.has_method("show_halftime"):
		event_overlay.show_halftime()


## 세컨드하프 오버레이 표시
func show_secondhalf_overlay() -> void:
	## NES 스타일에는 second half 텍스트가 없으므로 Socceralia 사용
	if event_overlay and event_overlay.has_method("show_secondhalf"):
		event_overlay.show_secondhalf()


## 타임업 오버레이 표시 (풀타임)
func show_timesup_overlay() -> void:
	if overlay_style == OverlayStyle.NES and nes_text_overlay:
		if nes_text_overlay.has_method("show_full_time"):
			nes_text_overlay.show_full_time()
	elif event_overlay and event_overlay.has_method("show_timesup"):
		event_overlay.show_timesup()


## 킥오프 오버레이 표시
func show_kickoff_overlay() -> void:
	## NES 스타일에는 kickoff 텍스트가 없으므로 Socceralia 사용
	if event_overlay and event_overlay.has_method("show_kickoff"):
		event_overlay.show_kickoff()


## 오프사이드 오버레이 표시 (NES 전용)
func show_offside_overlay() -> void:
	if overlay_style == OverlayStyle.NES and nes_text_overlay:
		if nes_text_overlay.has_method("show_offside"):
			nes_text_overlay.show_offside()


## 이벤트 오버레이 숨기기
func hide_event_overlay() -> void:
	if event_overlay and event_overlay.has_method("hide_overlay"):
		event_overlay.hide_overlay()
	if nes_text_overlay and nes_text_overlay.has_method("hide_overlay"):
		nes_text_overlay.hide_overlay()


## 스코어 설정
func set_score(home: int, away: int) -> void:
	if score_panel and score_panel.has_method("set_score"):
		score_panel.set_score(home, away)


## 팀 이름 설정 (스코어 패널)
func set_hud_team_names(home: String, away: String) -> void:
	if score_panel and score_panel.has_method("set_team_names"):
		score_panel.set_team_names(home, away)


## 경기 시간 설정 (밀리초)
func set_match_time(time_ms: int) -> void:
	if score_panel and score_panel.has_method("set_time_ms"):
		score_panel.set_time_ms(time_ms)
	# 이벤트 중복 방지용 시간 추적 업데이트 (2025-12-09)
	if event_overlay and event_overlay.has_method("update_playback_time"):
		event_overlay.update_playback_time(time_ms)


## 이벤트 중복 방지 리셋 (새 경기/타임라인 시작 시) (2025-12-09)
func reset_event_tracking() -> void:
	if event_overlay and event_overlay.has_method("reset_event_tracking"):
		event_overlay.reset_event_tracking()


## 이벤트로부터 골 오버레이 트리거 (중복 방지 적용) (2025-12-09)
func trigger_goal_from_event(event: Dictionary, team_name: String = "", scorer_name: String = "") -> void:
	if event_overlay and event_overlay.has_method("trigger_goal_from_event"):
		event_overlay.trigger_goal_from_event(event, team_name, scorer_name)
		trigger_goal_effect()
		# 득점자 세레모니 트리거 (2025-12-09 추가)
		_trigger_scorer_celebration(event)
	else:
		# Fallback: 기존 방식 (중복 방지 없음)
		show_goal_overlay(team_name, scorer_name)


## 득점자 세레모니 트리거 (2025-12-09 추가)
func _trigger_scorer_celebration(event: Dictionary) -> void:
	## 득점팀 추출 (team: "home"/"away" 또는 team_id: 0/1)
	var team_str: String = str(event.get("team", ""))
	var team_id: int = int(event.get("team_id", -1))
	if team_id < 0:
		if team_str == "home":
			team_id = 0
		elif team_str == "away":
			team_id = 1

	## 선수 인덱스 추출 (player_index 또는 player_id에서 파싱)
	var player_index: int = int(event.get("player_index", event.get("scorer_index", -1)))

	# Event SSOT: prefer player_track_id (0..21)
	var player_track_id: int = int(event.get(MatchEventKeys.PLAYER_TRACK_ID, -1))
	if player_track_id >= 0:
		# track_id implies team + local index
		team_id = 0 if player_track_id < 11 else 1
		player_index = player_track_id if team_id == 0 else player_track_id - 11
	elif player_index < 0:
		# Legacy fallback: parse from string id/name
		var player_id: String = str(event.get("player_id", event.get(MatchEventKeys.PLAYER_NAME, "")))
		player_index = _parse_player_index_from_id(player_id)

	if team_id < 0 or player_index < 0:
		return  # 정보 부족 시 스킵

	## 선수 배열에서 해당 선수 찾기
	var player_array: Array = _home_players if team_id == 0 else _away_players
	if player_index >= 0 and player_index < player_array.size():
		var scorer = player_array[player_index]
		if scorer and scorer.has_method("start_celebration"):
			scorer.start_celebration(3.0)


## player_id 문자열에서 인덱스 파싱 (2025-12-09 추가)
## 형식: "home_0", "away_5", "player_3", 숫자만 등
func _parse_player_index_from_id(player_id: String) -> int:
	if player_id.is_empty():
		return -1
	## "home_N" 또는 "away_N" 형식
	if player_id.begins_with("home_"):
		var idx_str := player_id.substr(5)
		return int(idx_str) if idx_str.is_valid_int() else -1
	elif player_id.begins_with("away_"):
		var idx_str := player_id.substr(5)
		return int(idx_str) if idx_str.is_valid_int() else -1
	## "player_N" 형식
	elif player_id.begins_with("player_"):
		var idx_str := player_id.substr(7)
		return int(idx_str) if idx_str.is_valid_int() else -1
	## 마지막 숫자 추출 시도 (예: "Striker10" → 10)
	elif player_id.is_valid_int():
		return int(player_id)
	else:
		## 문자열 끝의 숫자 추출
		var regex := RegEx.new()
		regex.compile("(\\d+)$")
		var result := regex.search(player_id)
		if result:
			return int(result.get_string(1))
	return -1


## 이벤트로부터 하프타임 오버레이 트리거 (중복 방지 적용) (2025-12-09)
func trigger_halftime_from_event(event: Dictionary) -> void:
	if event_overlay and event_overlay.has_method("trigger_halftime_from_event"):
		event_overlay.trigger_halftime_from_event(event)
	else:
		show_halftime_overlay()


## 이벤트로부터 풀타임 오버레이 트리거 (중복 방지 적용) (2025-12-09)
func trigger_fulltime_from_event(event: Dictionary) -> void:
	if event_overlay and event_overlay.has_method("trigger_fulltime_from_event"):
		event_overlay.trigger_fulltime_from_event(event)
	else:
		show_timesup_overlay()


#endregion

#region Match OS BoardOverlay Integration (2025-12-17)


## Setup Match OS BoardOverlay for debug visualization
func _setup_board_overlay() -> void:
	# Create overlay layer (above field, below HUD)
	var overlay_layer = CanvasLayer.new()
	overlay_layer.name = "MatchOSOverlayLayer"
	overlay_layer.layer = 5  # Between field (0) and HUD (10)
	add_child(overlay_layer)

	# Load BoardOverlay script
	var overlay_script = load("res://scripts/debug/BoardOverlay.gd")
	if overlay_script:
		_board_overlay = overlay_script.new()

		# Configure for horizontal viewer
		_board_overlay.scale_factor = 10.0  # 10px/m (matches HorizontalMatchViewer)
		_board_overlay.show_grid = true
		_board_overlay.show_heatmap = true
		_board_overlay.heatmap_mode = "pressure"  # Default mode
		_board_overlay.visible = false  # Hidden by default

		overlay_layer.add_child(_board_overlay)

		print("[HorizontalMatchViewer] BoardOverlay initialized (F3 to toggle)")
	else:
		push_warning("[HorizontalMatchViewer] Failed to load BoardOverlay.gd")


## Setup TeamView overlay for debug visualization
func _setup_team_view_overlay() -> void:
	var overlay_layer = CanvasLayer.new()
	overlay_layer.name = "TeamViewOverlayLayer"
	overlay_layer.layer = 6  # Above field, below HUD
	add_child(overlay_layer)

	var overlay_script = load("res://scripts/debug/TeamViewOverlay.gd")
	if overlay_script:
		_team_view_overlay = overlay_script.new()
		_team_view_overlay.visible = false
		overlay_layer.add_child(_team_view_overlay)
		print("[HorizontalMatchViewer] TeamViewOverlay initialized (F5 to toggle)")
	else:
		push_warning("[HorizontalMatchViewer] Failed to load TeamViewOverlay.gd")


## Setup DecisionIntent overlay for debug visualization
func _setup_decision_intent_overlay() -> void:
	var overlay_layer = CanvasLayer.new()
	overlay_layer.name = "DecisionIntentOverlayLayer"
	overlay_layer.layer = 7  # Above debug overlays, below HUD
	add_child(overlay_layer)

	var overlay_script = load("res://scripts/debug/DecisionIntentOverlay.gd")
	if overlay_script:
		_decision_intent_overlay = overlay_script.new()
		_decision_intent_overlay.scale_factor = METER_TO_PIXEL
		_decision_intent_overlay.visible = false
		overlay_layer.add_child(_decision_intent_overlay)
		print("[HorizontalMatchViewer] DecisionIntentOverlay initialized (F6 to toggle)")
	else:
		push_warning("[HorizontalMatchViewer] Failed to load DecisionIntentOverlay.gd")


## Toggle TeamView overlay visibility (call from input handler)
func toggle_team_view_overlay() -> void:
	if _team_view_overlay:
		_team_view_overlay_enabled = not _team_view_overlay_enabled
		_team_view_overlay.visible = _team_view_overlay_enabled
		_team_view_overlay.queue_redraw()
		print("[HorizontalMatchViewer] TeamViewOverlay: %s" % ("ON" if _team_view_overlay_enabled else "OFF"))


## Toggle BoardOverlay visibility (call from input handler)
func toggle_board_overlay() -> void:
	if _board_overlay:
		_board_overlay_enabled = not _board_overlay_enabled
		_board_overlay.visible = _board_overlay_enabled
		_board_overlay.is_visible_overlay = _board_overlay_enabled
		_board_overlay.queue_redraw()
		print("[HorizontalMatchViewer] BoardOverlay: %s" % ("ON" if _board_overlay_enabled else "OFF"))


## Toggle DecisionIntent overlay visibility (call from input handler)
func toggle_decision_intent_overlay() -> void:
	if _decision_intent_overlay:
		_decision_intent_overlay.toggle_visibility()
		_decision_intent_overlay_enabled = _decision_intent_overlay.is_visible_overlay


## Debug: Set debug mode (2025-12-22 FIX_2512)
func set_debug_mode(mode: DebugMode) -> void:
	_debug_mode = mode
	var mode_names = ["POSITION_ONLY", "EVENTS_ONLY", "FULL"]
	print("[HorizontalMatchViewer] Debug mode: %s" % mode_names[mode])


## Debug: Cycle through debug modes (2025-12-22 FIX_2512)
func cycle_debug_mode() -> DebugMode:
	_debug_mode = (_debug_mode + 1) % 3
	set_debug_mode(_debug_mode)
	return _debug_mode


## Input handler for F3/F4 toggle
func _input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed:
		# F3 - Toggle BoardOverlay
		if event.keycode == KEY_F3:
			toggle_board_overlay()
			get_viewport().set_input_as_handled()
		# F4 - Cycle debug mode (2025-12-22 FIX_2512)
		elif event.keycode == KEY_F4:
			cycle_debug_mode()
			get_viewport().set_input_as_handled()
		# F5 - Toggle TeamView overlay
		elif event.keycode == KEY_F5:
			toggle_team_view_overlay()
			get_viewport().set_input_as_handled()
		# F6 - Toggle DecisionIntent overlay
		elif event.keycode == KEY_F6:
			toggle_decision_intent_overlay()
			get_viewport().set_input_as_handled()

#endregion
