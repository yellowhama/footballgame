extends Node
class_name MatchSFXPlayer
##
## MatchSFXPlayer - 경기 사운드 효과 재생기
##
## 킥, 골, 휘슬 등 경기 중 사운드 효과를 관리합니다.
## 실제 오디오 파일이 없으면 플레이스홀더로 동작합니다.
##
## Created: 2025-12-11 (Phase 9)
##

#region Sound Effect Types
enum MatchSFX {
	# Ball Sounds
	BALL_KICK,  ## 킥/패스
	BALL_BOUNCE,  ## 공 바운스
	BALL_HIT_POST,  ## 골대 맞음
	# Goal Sounds
	GOAL_SCORED,  ## 골!
	GOAL_CELEBRATION,  ## 골 세레머니
	# Whistle Sounds
	WHISTLE_SHORT,  ## 짧은 휘슬 (파울, 오프사이드)
	WHISTLE_LONG,  ## 긴 휘슬 (킥오프, 하프타임)
	WHISTLE_TRIPLE,  ## 3연속 휘슬 (경기 종료)
	# Impact Sounds
	TACKLE,  ## 태클
	COLLISION,  ## 충돌
	HEADER,  ## 헤딩
	# Crowd Sounds
	CROWD_CHEER,  ## 환호
	CROWD_GROAN,  ## 탄식
	CROWD_AMBIENT,  ## 배경 함성 (루프)
	# Card Sounds
	CARD_YELLOW,  ## 옐로카드
	CARD_RED,  ## 레드카드
}
#endregion

#region Audio Players Pool
var _audio_players: Array[AudioStreamPlayer] = []
var _pool_size: int = 12
var _current_player_index: int = 0

## 루프 사운드용 별도 플레이어 (Crowd Ambient)
var _ambient_player: AudioStreamPlayer = null
#endregion

#region SFX Paths
## 실제 파일이 추가되면 이 경로에 맞춰 넣으면 됨
var _sfx_paths: Dictionary = {
	MatchSFX.BALL_KICK: "res://assets/audio/sfx/match/ball_kick.ogg",
	MatchSFX.BALL_BOUNCE: "res://assets/audio/sfx/match/ball_bounce.ogg",
	MatchSFX.BALL_HIT_POST: "res://assets/audio/sfx/match/ball_hit_post.ogg",
	MatchSFX.GOAL_SCORED: "res://assets/audio/sfx/match/goal_scored.ogg",
	MatchSFX.GOAL_CELEBRATION: "res://assets/audio/sfx/match/goal_celebration.ogg",
	MatchSFX.WHISTLE_SHORT: "res://assets/audio/sfx/match/whistle_short.ogg",
	MatchSFX.WHISTLE_LONG: "res://assets/audio/sfx/match/whistle_long.ogg",
	MatchSFX.WHISTLE_TRIPLE: "res://assets/audio/sfx/match/whistle_triple.ogg",
	MatchSFX.TACKLE: "res://assets/audio/sfx/match/tackle.ogg",
	MatchSFX.COLLISION: "res://assets/audio/sfx/match/collision.ogg",
	MatchSFX.HEADER: "res://assets/audio/sfx/match/header.ogg",
	MatchSFX.CROWD_CHEER: "res://assets/audio/sfx/match/crowd_cheer.ogg",
	MatchSFX.CROWD_GROAN: "res://assets/audio/sfx/match/crowd_groan.ogg",
	MatchSFX.CROWD_AMBIENT: "res://assets/audio/sfx/match/crowd_ambient.ogg",
	MatchSFX.CARD_YELLOW: "res://assets/audio/sfx/match/card_yellow.ogg",
	MatchSFX.CARD_RED: "res://assets/audio/sfx/match/card_red.ogg",
}
#endregion

#region Settings
var sfx_volume: float = 0.8
var sfx_enabled: bool = true
var _debug_log_missing: bool = true  ## 누락된 파일 로깅 (한 번만)
var _logged_missing: Dictionary = {}
#endregion


#region Lifecycle
func _ready() -> void:
	_create_audio_pool()
	_create_ambient_player()
	print("[MatchSFXPlayer] Initialized with %d audio players" % _pool_size)


func _create_audio_pool() -> void:
	for i in range(_pool_size):
		var player := AudioStreamPlayer.new()
		player.bus = "SFX"  ## Assumes SFX audio bus exists
		add_child(player)
		_audio_players.append(player)


func _create_ambient_player() -> void:
	_ambient_player = AudioStreamPlayer.new()
	_ambient_player.bus = "SFX"
	add_child(_ambient_player)


#endregion


#region Public API
## 사운드 효과 재생
func play(sfx_type: MatchSFX, volume_override: float = -1.0) -> void:
	if not sfx_enabled:
		return

	var sfx_path: String = _sfx_paths.get(sfx_type, "")

	## 파일 존재 확인
	if not ResourceLoader.exists(sfx_path):
		_log_missing(sfx_type, sfx_path)
		return

	var stream = load(sfx_path)
	if not stream:
		return

	var player := _get_next_player()
	var volume: float = volume_override if volume_override >= 0.0 else sfx_volume
	player.volume_db = linear_to_db(volume)
	player.stream = stream
	player.play()


## 이벤트 타입에 따른 사운드 자동 재생
func play_for_event(event_type: String) -> void:
	match event_type.to_lower():
		"goal":
			play(MatchSFX.GOAL_SCORED)
			## 약간 딜레이 후 환호
			await get_tree().create_timer(0.3).timeout
			play(MatchSFX.CROWD_CHEER)
		"shot", "shot_on_target", "shot_off_target":
			play(MatchSFX.BALL_KICK)
		"pass", "pass_complete":
			play(MatchSFX.BALL_KICK, 0.5)  ## 패스는 약간 작게
		"foul":
			play(MatchSFX.WHISTLE_SHORT)
			play(MatchSFX.CROWD_GROAN, 0.3)
		"yellow_card":
			play(MatchSFX.WHISTLE_SHORT)
			play(MatchSFX.CARD_YELLOW)
		"red_card":
			play(MatchSFX.WHISTLE_LONG)
			play(MatchSFX.CARD_RED)
		"save":
			play(MatchSFX.CROWD_CHEER, 0.6)
		"kickoff", "half_time":
			play(MatchSFX.WHISTLE_LONG)
		"full_time":
			play(MatchSFX.WHISTLE_TRIPLE)
		"header":
			play(MatchSFX.HEADER)
		"tackle":
			play(MatchSFX.TACKLE)


## 공 바운스 사운드
func play_ball_bounce() -> void:
	play(MatchSFX.BALL_BOUNCE, 0.6)


## 공이 골대 맞았을 때
func play_ball_hit_post() -> void:
	play(MatchSFX.BALL_HIT_POST)


## 배경 관중 함성 시작/정지
func start_crowd_ambient() -> void:
	if not sfx_enabled:
		return

	var sfx_path: String = _sfx_paths.get(MatchSFX.CROWD_AMBIENT, "")
	if not ResourceLoader.exists(sfx_path):
		_log_missing(MatchSFX.CROWD_AMBIENT, sfx_path)
		return

	var stream = load(sfx_path)
	if stream:
		_ambient_player.stream = stream
		_ambient_player.volume_db = linear_to_db(sfx_volume * 0.3)  ## 배경이므로 낮게
		_ambient_player.play()


func stop_crowd_ambient() -> void:
	if _ambient_player:
		_ambient_player.stop()


## 볼륨 설정
func set_volume(volume: float) -> void:
	sfx_volume = clamp(volume, 0.0, 1.0)


## 활성화/비활성화
func set_enabled(enabled: bool) -> void:
	sfx_enabled = enabled
	if not enabled and _ambient_player:
		_ambient_player.stop()


## 모든 사운드 정지
func stop_all() -> void:
	for player in _audio_players:
		if player.playing:
			player.stop()
	if _ambient_player:
		_ambient_player.stop()


#endregion


#region Internal
func _get_next_player() -> AudioStreamPlayer:
	var player := _audio_players[_current_player_index]
	_current_player_index = (_current_player_index + 1) % _pool_size
	return player


func _log_missing(sfx_type: MatchSFX, path: String) -> void:
	if not _debug_log_missing:
		return
	if _logged_missing.has(sfx_type):
		return
	_logged_missing[sfx_type] = true
	print("[MatchSFXPlayer] Missing SFX: %s (placeholder mode)" % path)
#endregion
