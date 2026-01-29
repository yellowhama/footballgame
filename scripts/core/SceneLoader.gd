extends Node

signal scene_loaded(scene_name: String)

var current_scene: Node
var loading_scene: String = ""


func _ready():
	print("SceneLoader initialized")
	# 현재 씬을 첫 번째 씬으로 설정
	current_scene = get_tree().current_scene


func load_scene(scene_path: String):
	"""씬을 로드하고 전환합니다"""
	print("Loading scene: ", scene_path)
	loading_scene = scene_path

	# 리소스 존재 확인
	if not ResourceLoader.exists(scene_path):
		push_error("Scene file not found: " + scene_path)
		return

	# 씬 로드
	var scene = load(scene_path)
	if not scene:
		push_error("Failed to load scene: " + scene_path)
		return

	# 새 씬 인스턴스 생성
	var new_scene = scene.instantiate()
	if not new_scene:
		push_error("Failed to instantiate scene: " + scene_path)
		return

	# 현재 씬 제거
	if current_scene:
		current_scene.queue_free()

	# 새 씬을 트리에 추가
	get_tree().root.add_child(new_scene)
	get_tree().current_scene = new_scene
	current_scene = new_scene

	# 신호 발생
	scene_loaded.emit(scene_path.get_file().get_basename())
	print("Scene loaded successfully: ", scene_path)


func load_scene_async(scene_path: String):
	"""비동기로 씬을 로드합니다"""
	print("Loading scene asynchronously: ", scene_path)

	# 리소스 로더를 사용한 비동기 로딩
	var loader = ResourceLoader.load_threaded_request(scene_path)
	if loader != OK:
		push_error("Failed to start async loading: " + scene_path)
		return

	# 로딩 완료까지 대기
	while true:
		var progress = ResourceLoader.load_threaded_get_status(scene_path)
		if progress == ResourceLoader.THREAD_LOAD_LOADED:
			break
		elif progress == ResourceLoader.THREAD_LOAD_FAILED:
			push_error("Async loading failed: " + scene_path)
			return
		await get_tree().process_frame

	# 씬 로드 완료
	var scene = ResourceLoader.load_threaded_get(scene_path)
	load_scene(scene_path)
