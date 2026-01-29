extends CanvasLayer
class_name EventOverlay
##
## EventOverlay - 골 애니메이션 및 이벤트 오버레이
##
## 기능:
##   - 골 득점 시 애니메이션 (goal_animation-1~9.png)
##   - 하프타임/세컨드하프/타임업 오버레이
##   - 킥오프 오버레이
##
## 스펙: docs/spec+@/spec_v4/dev_spec/UI/2025-12-07_SOCCERALIA_HORIZONTAL_VIEW_SPEC.md
##

#region Constants
const HUD_PATH := "res://assets/sprites/socceralia/hud/"
const SFX_PATH := "res://assets/audio/sfx/"
const GOAL_ANIMATION_FRAMES := 9
const GOAL_ANIMATION_FPS := 12.0
const GOAL_DISPLAY_DURATION := 3.0
const EVENT_DISPLAY_DURATION := 2.5

## 액션 팝업 (pass, dribble 등)
const ACTION_POPUP_DURATION := 0.8
const ACTION_LABELS := {
	"pass": "PASS",
	"dribble": "DRIBBLE",
	"shot_on_target": "SHOT",
	"shot": "SHOT",
	"save": "SAVE",
	"tackle": "TACKLE",
	"key_chance": "CHANCE"
}
#endregion

#region SFX Paths
## 사운드 파일 경로 (파일이 없으면 무시됨)
const SFX_GOAL := "res://assets/audio/sfx/goal.ogg"
const SFX_WHISTLE := "res://assets/audio/sfx/whistle.ogg"
const SFX_FOUL := "res://assets/audio/sfx/foul.ogg"
const SFX_YELLOW_CARD := "res://assets/audio/sfx/card.ogg"
const SFX_RED_CARD := "res://assets/audio/sfx/card.ogg"
const SFX_CROWD_CHEER := "res://assets/audio/sfx/crowd_cheer.ogg"
const SFX_HALFTIME := "res://assets/audio/sfx/whistle_long.ogg"
#endregion

#region Node References
var _overlay_container: CenterContainer = null
var _goal_sprite: AnimatedSprite2D = null
var _event_sprite: Sprite2D = null
var _background: ColorRect = null
var _action_label: Label = null  ## 액션 팝업 라벨 (PASS, DRIBBLE 등)
var _action_tween: Tween = null  ## 액션 팝업 애니메이션
#endregion

#region Textures
var _goal_frames: Array[Texture2D] = []
var _halftime_texture: Texture2D = null
var _secondhalf_texture: Texture2D = null
var _timesup_texture: Texture2D = null
var _kickoff_texture: Texture2D = null
var _goal_bg_texture: Texture2D = null
#endregion

#region State
var _is_showing: bool = false
var _sfx_cache: Dictionary = {}  ## 캐시된 SFX AudioStream
var _processed_event_ids: Dictionary = {}  ## 이벤트 중복 방지 (2025-12-09)
var _last_event_direction: int = 1  ## 1=forward, -1=backward (seek 방향 추적)
var _last_processed_time_ms: int = 0  ## 마지막 처리된 시간
#endregion

#region Signals
signal overlay_finished
signal goal_animation_started
signal goal_animation_finished
#endregion


func _ready() -> void:
	layer = 100  # CanvasLayer - 최상위
	_load_textures()
	_load_sfx()
	_create_ui_elements()
	_hide_all()


func _load_textures() -> void:
	## 골 애니메이션 프레임 로드 (1~9)
	for i in range(1, GOAL_ANIMATION_FRAMES + 1):
		var path := HUD_PATH + "goal_animation-%d.png" % i
		if ResourceLoader.exists(path):
			_goal_frames.append(load(path))

	## 이벤트 텍스처 로드
	var textures := {
		"halftime": "_halftime_texture",
		"secondhalf": "_secondhalf_texture",
		"timesup": "_timesup_texture",
		"kickoff": "_kickoff_texture",
		"goal_bg-1": "_goal_bg_texture",
	}

	for filename in textures:
		var path: String = HUD_PATH + filename + ".png"
		if ResourceLoader.exists(path):
			set(textures[filename], load(path))


func _load_sfx() -> void:
	## SFX 에셋 로드 (존재하는 파일만)
	var sfx_paths := {
		"goal": SFX_GOAL,
		"whistle": SFX_WHISTLE,
		"foul": SFX_FOUL,
		"yellow_card": SFX_YELLOW_CARD,
		"red_card": SFX_RED_CARD,
		"crowd_cheer": SFX_CROWD_CHEER,
		"halftime": SFX_HALFTIME,
	}

	for key in sfx_paths:
		var path: String = sfx_paths[key]
		if ResourceLoader.exists(path):
			_sfx_cache[key] = load(path)


func _play_sfx(sfx_key: String) -> void:
	## SFX 재생 (SoundManager 싱글톤 사용)
	if not _sfx_cache.has(sfx_key):
		return

	var sound_manager = Engine.get_singleton("SoundManager")
	if sound_manager == null and has_node("/root/SoundManager"):
		sound_manager = get_node("/root/SoundManager")

	if sound_manager and sound_manager.has_method("play_sound"):
		sound_manager.play_sound(_sfx_cache[sfx_key])


func _create_ui_elements() -> void:
	## 배경 (반투명 검정)
	_background = ColorRect.new()
	_background.name = "Background"
	_background.color = Color(0, 0, 0, 0.5)
	_background.set_anchors_preset(Control.PRESET_FULL_RECT)
	_background.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(_background)

	## 중앙 정렬 컨테이너
	_overlay_container = CenterContainer.new()
	_overlay_container.name = "OverlayContainer"
	_overlay_container.set_anchors_preset(Control.PRESET_FULL_RECT)
	_overlay_container.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(_overlay_container)

	## 골 애니메이션 스프라이트
	_goal_sprite = AnimatedSprite2D.new()
	_goal_sprite.name = "GoalAnimation"
	_goal_sprite.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	_goal_sprite.scale = Vector2(4.0, 4.0)  # 픽셀아트 확대
	_setup_goal_animation()
	_overlay_container.add_child(_goal_sprite)

	## 이벤트 텍스트 스프라이트 (halftime, timesup 등)
	_event_sprite = Sprite2D.new()
	_event_sprite.name = "EventText"
	_event_sprite.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
	_event_sprite.scale = Vector2(3.0, 3.0)  # 픽셀아트 확대
	_overlay_container.add_child(_event_sprite)

	## 액션 팝업 라벨 (PASS, DRIBBLE 등)
	_create_action_label()


func _setup_goal_animation() -> void:
	if _goal_frames.is_empty():
		return

	var sprite_frames := SpriteFrames.new()
	sprite_frames.add_animation("goal")
	sprite_frames.set_animation_speed("goal", GOAL_ANIMATION_FPS)
	sprite_frames.set_animation_loop("goal", false)

	for i in range(_goal_frames.size()):
		sprite_frames.add_frame("goal", _goal_frames[i])

	_goal_sprite.sprite_frames = sprite_frames
	_goal_sprite.animation_finished.connect(_on_goal_animation_finished)


func _hide_all() -> void:
	_background.visible = false
	_goal_sprite.visible = false
	_event_sprite.visible = false
	_is_showing = false


#region Public API


## 골 애니메이션 표시
func show_goal(team_name: String = "", scorer_name: String = "") -> void:
	if _is_showing:
		return

	_is_showing = true
	_background.visible = true
	_background.color = Color(0, 0, 0, 0.6)

	_event_sprite.visible = false
	_goal_sprite.visible = true
	_goal_sprite.frame = 0
	_goal_sprite.play("goal")

	## SFX 재생
	_play_sfx("goal")
	_play_sfx("crowd_cheer")

	goal_animation_started.emit()

	## 애니메이션 끝나도 잠시 유지
	await get_tree().create_timer(GOAL_DISPLAY_DURATION).timeout
	_hide_all()
	goal_animation_finished.emit()
	overlay_finished.emit()


## 하프타임 오버레이
func show_halftime() -> void:
	_play_sfx("halftime")
	await _show_event_overlay(_halftime_texture, EVENT_DISPLAY_DURATION + 1.0)


## 세컨드하프 오버레이
func show_secondhalf() -> void:
	_play_sfx("whistle")
	await _show_event_overlay(_secondhalf_texture, EVENT_DISPLAY_DURATION)


## 타임업 오버레이
func show_timesup() -> void:
	_play_sfx("halftime")  ## 긴 휘슬
	await _show_event_overlay(_timesup_texture, EVENT_DISPLAY_DURATION + 1.0)


## 킥오프 오버레이
func show_kickoff() -> void:
	_play_sfx("whistle")
	await _show_event_overlay(_kickoff_texture, EVENT_DISPLAY_DURATION)


## 파울 이벤트 (SFX만)
func play_foul_sfx() -> void:
	_play_sfx("whistle")
	_play_sfx("foul")


## 옐로카드 이벤트 (SFX만)
func play_yellow_card_sfx() -> void:
	_play_sfx("whistle")
	_play_sfx("yellow_card")


## 레드카드 이벤트 (SFX만)
func play_red_card_sfx() -> void:
	_play_sfx("whistle")
	_play_sfx("red_card")


## 이벤트 오버레이 숨기기
func hide_overlay() -> void:
	_hide_all()


## 현재 오버레이 표시 중인지
func is_showing() -> bool:
	return _is_showing


#endregion

#region Internal


func _show_event_overlay(texture: Texture2D, duration: float) -> void:
	if _is_showing or texture == null:
		return

	_is_showing = true
	_background.visible = true
	_background.color = Color(0, 0, 0, 0.5)

	_goal_sprite.visible = false
	_event_sprite.visible = true
	_event_sprite.texture = texture

	await get_tree().create_timer(duration).timeout
	_hide_all()
	overlay_finished.emit()


func _on_goal_animation_finished() -> void:
	## 애니메이션 완료 후 마지막 프레임 유지
	pass


func _create_action_label() -> void:
	## 화면 하단에 액션 팝업 표시용 라벨 생성
	## CanvasLayer에는 크기가 없으므로 전체 화면 Control을 먼저 생성
	var action_container := Control.new()
	action_container.name = "ActionContainer"
	action_container.set_anchors_preset(Control.PRESET_FULL_RECT)
	action_container.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(action_container)

	_action_label = Label.new()
	_action_label.name = "ActionLabel"
	_action_label.add_theme_font_size_override("font_size", 24)
	_action_label.add_theme_color_override("font_color", Color.YELLOW)
	_action_label.add_theme_color_override("font_shadow_color", Color(0, 0, 0, 0.9))
	_action_label.add_theme_constant_override("shadow_offset_x", 2)
	_action_label.add_theme_constant_override("shadow_offset_y", 2)
	_action_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_action_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	_action_label.set_anchors_preset(Control.PRESET_CENTER_BOTTOM)
	_action_label.anchor_bottom = 0.85  ## 화면 85% 높이에 위치
	_action_label.anchor_top = 0.85
	_action_label.grow_horizontal = Control.GROW_DIRECTION_BOTH
	_action_label.visible = false
	action_container.add_child(_action_label)


#endregion

#region Action Popup (2025-12-10)


## 액션 팝업 표시 (PASS, DRIBBLE 등)
func show_action(action_type: String, player_name: String = "") -> void:
	print("[EventOverlay] show_action called: action_type=%s, player=%s" % [action_type, player_name])
	if not ACTION_LABELS.has(action_type):
		print("[EventOverlay] action_type '%s' not in ACTION_LABELS, skipping" % action_type)
		return
	if _action_label == null:
		print("[EventOverlay] _action_label is null!")
		return

	## 이전 애니메이션 취소
	if _action_tween and _action_tween.is_valid():
		_action_tween.kill()

	var text: String = ACTION_LABELS[action_type]
	if player_name != "":
		text = "%s - %s" % [player_name, text]

	print("[EventOverlay] Showing action popup: %s" % text)
	_action_label.text = text
	_action_label.visible = true
	_action_label.modulate.a = 1.0

	## 페이드 아웃 애니메이션
	_action_tween = create_tween()
	_action_tween.tween_property(_action_label, "modulate:a", 0.0, ACTION_POPUP_DURATION)
	_action_tween.tween_callback(func(): _action_label.visible = false)


## 이벤트 딕셔너리에서 액션 트리거
## ✅ P0-2 (2025-12-22 FIX_2512): Strengthened validation with debug logging
func trigger_action_from_event(event: Dictionary) -> void:
	## 다양한 이벤트 구조 지원: kind (Rust), type, etype, label
	var action_type: String = (
		str(event.get("kind", event.get("etype", event.get("type", event.get("label", ""))))).strip_edges().to_lower()
	)

	## ✅ P0-2: Stricter validation - must have valid action type in ACTION_LABELS
	if action_type == "" or action_type == "unknown" or action_type == "n/a" or not ACTION_LABELS.has(action_type):
		if OS.is_debug_build():
			print("[EventOverlay] Rejected: invalid action_type='%s'" % action_type)
		return

	## player_id는 base 내부 또는 루트에 있을 수 있음
	var base: Dictionary = event.get("base", {})
	var player_id = base.get("player_id", event.get("player_id", event.get(MatchEventKeys.PLAYER_NAME, null)))

	## SSOT: actor track_id may exist even when display name is missing
	var player_tid: int = int(event.get(MatchEventKeys.PLAYER_TRACK_ID, -1))
	if player_tid < 0 and base.has(MatchEventKeys.PLAYER_TRACK_ID):
		player_tid = int(base.get(MatchEventKeys.PLAYER_TRACK_ID, -1))

	var player_name: String = "" if player_id == null else str(player_id).strip_edges()

	## ✅ P0-2: Stricter validation - must have actor (either player_name or track_id)
	if player_name == "" and player_tid < 0:
		if OS.is_debug_build():
			print("[EventOverlay] Rejected: null player_id/track_id for action=%s" % action_type)
		return

	print("[EventOverlay] trigger_action_from_event: kind=%s, action_type=%s" % [event.get("kind", "N/A"), action_type])
	show_action(action_type, player_name)


#endregion

#region Event Deduplication (2025-12-09)


## 이벤트 ID 계산 (type + t_ms + team_id + player 조합)
func _compute_event_id(event: Dictionary) -> String:
	var event_type: String = str(event.get(MatchEventKeys.TYPE, event.get("kind", "unknown")))
	var t_ms: int = int(event.get(MatchEventKeys.T_MS, 0))
	var team_id: int = int(event.get("team_id", -1))
	var player_tid: int = int(event.get(MatchEventKeys.PLAYER_TRACK_ID, -1))
	var target_tid: int = int(event.get(MatchEventKeys.TARGET_TRACK_ID, -1))
	# Match OS: event identity is SSOT-only (track_id based). Values may be -1.
	return "%s_%d_%d_%d_%d" % [event_type, t_ms, team_id, player_tid, target_tid]


## 이벤트가 이미 처리되었는지 확인 (중복 방지)
func should_trigger_event(event: Dictionary) -> bool:
	var event_id := _compute_event_id(event)
	if _processed_event_ids.has(event_id):
		return false
	_processed_event_ids[event_id] = true
	return true


## 이벤트 추적 리셋 (새 경기 시작 또는 타임라인 처음으로 돌아갈 때)
func reset_event_tracking() -> void:
	_processed_event_ids.clear()
	_last_processed_time_ms = 0


## 뒤로 감기(seek backward) 시 호출 - 해당 시간 이후 이벤트 추적 제거
func clear_events_after(time_ms: int) -> void:
	var keys_to_remove: Array = []
	for event_id in _processed_event_ids.keys():
		# event_id 포맷: "type_t_ms_team_id_player"
		var parts: PackedStringArray = str(event_id).split("_")
		if parts.size() >= 2:
			var event_t_ms: int = int(parts[1])
			if event_t_ms > time_ms:
				keys_to_remove.append(event_id)
	for key in keys_to_remove:
		_processed_event_ids.erase(key)
	_last_processed_time_ms = time_ms


## 시간 업데이트 시 방향 추적 (뒤로 감기 감지)
func update_playback_time(time_ms: int) -> void:
	if time_ms < _last_processed_time_ms - 500:  # 500ms 이상 뒤로 갔으면
		clear_events_after(time_ms)
	_last_processed_time_ms = time_ms


## 이벤트로부터 골 오버레이 트리거 (중복 방지 적용)
func trigger_goal_from_event(event: Dictionary, team_name: String = "", scorer_name: String = "") -> void:
	if not should_trigger_event(event):
		return  # 이미 표시된 이벤트
	show_goal(team_name, scorer_name)


## 이벤트로부터 하프타임 오버레이 트리거 (중복 방지 적용)
func trigger_halftime_from_event(event: Dictionary) -> void:
	if not should_trigger_event(event):
		return
	show_halftime()


## 이벤트로부터 풀타임 오버레이 트리거 (중복 방지 적용)
func trigger_fulltime_from_event(event: Dictionary) -> void:
	if not should_trigger_event(event):
		return
	show_timesup()

#endregion
