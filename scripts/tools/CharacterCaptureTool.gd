extends Node
class_name CharacterCaptureTool
## SubViewport를 사용해 CutoutPlayer를 PNG로 캡처하는 유틸리티
## - 개별 프레임 캡처
## - 애니메이션별 스프라이트 시트 생성
## - UI 미리보기용 실시간 렌더링

# 설정
@export var capture_size := Vector2i(128, 128)  # 캡처 크기
@export var output_dir := "user://captured/"  # 출력 폴더
@export var transparent_bg := true  # 투명 배경

# 내부
var _viewport: SubViewport
var _camera: Camera2D
var _player: Node2D
var _is_capturing := false

signal capture_complete(path: String)
signal sheet_complete(path: String, frame_count: int)


func _ready() -> void:
	_setup_viewport()


func _setup_viewport() -> void:
	# SubViewport 생성
	_viewport = SubViewport.new()
	_viewport.size = capture_size
	_viewport.transparent_bg = transparent_bg
	_viewport.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	_viewport.render_target_clear_mode = SubViewport.CLEAR_MODE_ALWAYS
	add_child(_viewport)

	# Camera2D 생성
	_camera = Camera2D.new()
	_camera.anchor_mode = Camera2D.ANCHOR_MODE_FIXED_TOP_LEFT
	_camera.offset = Vector2(capture_size.x / 2.0, capture_size.y * 0.8)  # 캐릭터 중앙
	_viewport.add_child(_camera)


func set_player(player_node: Node2D) -> void:
	"""캡처할 플레이어 노드 설정 (CutoutPlayer 인스턴스)"""
	if _player and _player.get_parent() == _viewport:
		_viewport.remove_child(_player)

	_player = player_node
	if _player.get_parent():
		_player.get_parent().remove_child(_player)
	_viewport.add_child(_player)
	_player.position = Vector2(capture_size.x / 2.0, capture_size.y * 0.7)


func capture_single_frame(filename: String = "") -> String:
	"""현재 프레임을 PNG로 캡처"""
	await RenderingServer.frame_post_draw

	var image := _viewport.get_texture().get_image()

	if filename.is_empty():
		filename = "frame_%d.png" % Time.get_ticks_msec()

	var full_path := output_dir.path_join(filename)
	_ensure_dir(output_dir)

	var err := image.save_png(full_path)
	if err == OK:
		capture_complete.emit(full_path)
		return full_path
	else:
		push_error("Failed to save PNG: %s" % full_path)
		return ""


func capture_animation_sheet(anim_name: String, frame_interval: float = 0.1) -> String:
	"""애니메이션을 스프라이트 시트로 캡처"""
	if not _player or not _player.has_method("play_action"):
		push_error("No player set or player doesn't support play_action")
		return ""

	if _is_capturing:
		push_warning("Already capturing")
		return ""

	_is_capturing = true

	# 애니메이션 정보 확인
	var anim_player: AnimationPlayer = _player.get_node_or_null("AnimationPlayer")
	if not anim_player or not anim_player.has_animation(anim_name):
		push_error("Animation not found: %s" % anim_name)
		_is_capturing = false
		return ""

	var anim: Animation = anim_player.get_animation(anim_name)
	var duration: float = anim.length
	var frame_count: int = int(ceil(duration / frame_interval))

	# 프레임들 캡처
	var frames: Array[Image] = []

	anim_player.play(anim_name)
	anim_player.pause()

	for i in range(frame_count):
		var time := i * frame_interval
		anim_player.seek(time, true)

		# 렌더링 대기
		await get_tree().process_frame
		await RenderingServer.frame_post_draw

		var frame_image := _viewport.get_texture().get_image()
		frames.append(frame_image)

	anim_player.stop()

	# 스프라이트 시트 조합
	var sheet_width := capture_size.x * frame_count
	var sheet_height := capture_size.y
	var sheet := Image.create(sheet_width, sheet_height, false, Image.FORMAT_RGBA8)

	for i in range(frames.size()):
		var frame := frames[i]
		var dest_rect := Rect2i(i * capture_size.x, 0, capture_size.x, capture_size.y)
		sheet.blit_rect(frame, Rect2i(Vector2i.ZERO, capture_size), dest_rect.position)

	# 저장
	var filename := "%s_sheet_%dx%d.png" % [anim_name, frame_count, 1]
	var full_path := output_dir.path_join(filename)
	_ensure_dir(output_dir)

	var err := sheet.save_png(full_path)
	_is_capturing = false

	if err == OK:
		sheet_complete.emit(full_path, frame_count)
		return full_path
	else:
		push_error("Failed to save sheet: %s" % full_path)
		return ""


func capture_all_animations(frame_interval: float = 0.1) -> Dictionary:
	"""모든 애니메이션을 시트로 캡처"""
	var results := {}

	var animations := ["idle", "run", "kick", "tackle", "heading", "celebrate"]

	for anim_name in animations:
		var path := await capture_animation_sheet(anim_name, frame_interval)
		if not path.is_empty():
			results[anim_name] = path

		# 다음 캡처 전 대기
		await get_tree().create_timer(0.1).timeout

	return results


func capture_direction_variants(direction_count: int = 8) -> Dictionary:
	"""8방향 idle 프레임 캡처"""
	if not _player or not _player.has_method("_update_facing"):
		push_error("Player doesn't support direction changes")
		return {}

	var results := {}
	var direction_names := ["E", "NE", "N", "NW", "W", "SW", "S", "SE"]

	for i in range(min(direction_count, 8)):
		# 방향 설정
		_player.call("_update_facing", i)

		await get_tree().process_frame
		await RenderingServer.frame_post_draw

		var filename := "dir_%s.png" % direction_names[i]
		var path := await capture_single_frame(filename)
		if not path.is_empty():
			results[direction_names[i]] = path

	return results


func get_preview_texture() -> ViewportTexture:
	"""UI에서 실시간으로 사용할 뷰포트 텍스처 반환"""
	return _viewport.get_texture()


func _ensure_dir(dir_path: String) -> void:
	"""폴더가 없으면 생성"""
	if not DirAccess.dir_exists_absolute(dir_path):
		DirAccess.make_dir_recursive_absolute(dir_path)


# ==============================================================================
# Batch Processing - 여러 외형 조합 캡처
# ==============================================================================


func batch_capture_variants(team_colors: Array, skin_tones: Array, hair_colors: Array) -> int:  # [{"primary": "red", "secondary": "white"}, ...]  # [0.7, 0.8, 0.9, 1.0]  # [Color(0.1, 0.05, 0.02), ...]
	"""여러 외형 조합을 배치 캡처"""
	if not _player:
		push_error("No player set")
		return 0

	var count := 0
	var batch_dir := output_dir.path_join("batch_%d" % Time.get_ticks_msec())
	_ensure_dir(batch_dir)

	for tc in team_colors:
		for st in skin_tones:
			for hc in hair_colors:
				# 외형 적용
				if _player.has_method("set_team_colors"):
					_player.set_team_colors(tc.get("primary", "red"), tc.get("secondary", "white"))

				if _player.has_method("set_appearance"):
					_player.set_appearance({"skin_tone": st, "hair_color": hc})

				await get_tree().process_frame
				await RenderingServer.frame_post_draw

				# 캡처
				var filename := "var_%s_%s_%.1f.png" % [tc.get("primary", "unk"), str(hc).replace(",", "_"), st]
				var saved_path := batch_dir.path_join(filename)

				var image := _viewport.get_texture().get_image()
				if image.save_png(saved_path) == OK:
					count += 1

	print("Batch capture complete: %d images saved to %s" % [count, batch_dir])
	return count
