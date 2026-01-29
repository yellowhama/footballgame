extends Node
class_name TouchOptimizer

# 터치 UI 최적화 시스템
# 터치 타겟 크기 자동 조정, 터치 피드백, 성능 최적화 등

signal touch_target_optimized(node: Node, old_size: Vector2, new_size: Vector2)
signal touch_feedback_applied(node: Node, feedback_type: String)

# 최적화 설정
const MIN_TOUCH_SIZE = 44  # 최소 터치 타겟 크기 (px)
const PREFERRED_TOUCH_SIZE = 60  # 권장 터치 타겟 크기 (px)
const MIN_SPACING = 16  # 최소 UI 요소 간격 (px)
const MIN_FONT_SIZE = 16  # 최소 폰트 크기 (px)

enum TouchFeedbackType { NONE, SCALE, COLOR, VIBRATION, SOUND, COMBINED }  # 크기 변화  # 색상 변화  # 진동  # 소리  # 복합 피드백

var optimized_nodes: Array = []
var feedback_tweens: Dictionary = {}


func _ready():
	# 최적화 프로세스 시작
	call_deferred("optimize_all_scenes")


func optimize_all_scenes():
	# 모든 씬의 UI 요소들을 최적화
	var scenes = get_tree().get_nodes_in_group("ui_scenes")
	for scene in scenes:
		optimize_scene(scene)


func optimize_scene(scene: Node):
	print("Optimizing scene: ", scene.name)

	# 터치 타겟 크기 최적화
	optimize_touch_targets(scene)

	# 폰트 크기 최적화
	optimize_font_sizes(scene)

	# 간격 최적화
	optimize_spacing(scene)

	# 터치 피드백 설정
	setup_touch_feedback(scene)


func optimize_touch_targets(scene: Node):
	var buttons = []
	find_buttons_recursive(scene, buttons)

	for button in buttons:
		optimize_button_size(button)


func optimize_button_size(button: Button):
	var current_size = button.custom_minimum_size
	var needs_optimization = false
	var new_size = current_size

	# 최소 크기 확인
	if current_size.x < MIN_TOUCH_SIZE:
		new_size.x = MIN_TOUCH_SIZE
		needs_optimization = true

	if current_size.y < MIN_TOUCH_SIZE:
		new_size.y = MIN_TOUCH_SIZE
		needs_optimization = true

	# 중요한 버튼은 더 크게 (확인, 실행 등)
	if is_important_button(button):
		if new_size.x < PREFERRED_TOUCH_SIZE:
			new_size.x = PREFERRED_TOUCH_SIZE
			needs_optimization = true

	if needs_optimization:
		button.custom_minimum_size = new_size
		optimized_nodes.append(button)
		emit_signal("touch_target_optimized", button, current_size, new_size)
		print("Optimized button: ", button.name, " from ", current_size, " to ", new_size)


func is_important_button(button: Button) -> bool:
	var button_text = button.text.to_lower()
	var important_keywords = [
		"확인", "실행", "저장", "계속", "시작", "완료", "ok", "execute", "save", "continue", "start", "finish"
	]

	for keyword in important_keywords:
		if keyword in button_text:
			return true

	# 이름으로도 확인
	var button_name = button.name.to_lower()
	var important_names = ["confirm", "execute", "save", "continue", "start", "finish"]

	for name in important_names:
		if name in button_name:
			return true

	return false


func optimize_font_sizes(scene: Node):
	var text_nodes = []
	find_text_nodes_recursive(scene, text_nodes)

	for node in text_nodes:
		optimize_text_size(node)


func optimize_text_size(node: Node):
	if not node.has_method("get_theme_font_size"):
		return

	var current_size = get_effective_font_size(node)

	if current_size < MIN_FONT_SIZE:
		node.add_theme_font_size_override("font_size", MIN_FONT_SIZE)
		print("Optimized font size for: ", node.name, " from ", current_size, " to ", MIN_FONT_SIZE)


func get_effective_font_size(node: Node) -> int:
	# 현재 적용된 폰트 크기 가져오기
	if node.has_method("get_theme_font_size"):
		var size = node.get_theme_font_size("font_size")
		if size > 0:
			return size

	# 기본값 반환
	if node is Button:
		return 16
	elif node is Label:
		return 14
	else:
		return 12


func optimize_spacing(scene: Node):
	var containers = []
	find_containers_recursive(scene, containers)

	for container in containers:
		optimize_container_spacing(container)


func optimize_container_spacing(container: Container):
	if container.has_method("add_theme_constant_override"):
		# 현재 separation 값 확인
		var current_separation = container.get_theme_constant("separation")

		if current_separation < MIN_SPACING:
			container.add_theme_constant_override("separation", MIN_SPACING)
			print("Optimized spacing for: ", container.name, " from ", current_separation, " to ", MIN_SPACING)

		# GridContainer의 경우 h_separation, v_separation도 확인
		if container is GridContainer:
			var h_sep = container.get_theme_constant("h_separation")
			var v_sep = container.get_theme_constant("v_separation")

			if h_sep < MIN_SPACING:
				container.add_theme_constant_override("h_separation", MIN_SPACING)

			if v_sep < MIN_SPACING:
				container.add_theme_constant_override("v_separation", MIN_SPACING)


func setup_touch_feedback(scene: Node):
	var interactive_nodes = []
	find_interactive_nodes_recursive(scene, interactive_nodes)

	for node in interactive_nodes:
		setup_node_feedback(node)


func setup_node_feedback(node: Node):
	if node is Button:
		setup_button_feedback(node)
	elif node is LineEdit:
		setup_input_feedback(node)


func setup_button_feedback(button: Button):
	# 기존 연결 제거 (중복 방지)
	if button.pressed.is_connected(_on_button_pressed):
		button.pressed.disconnect(_on_button_pressed)

	# 터치 피드백 연결
	button.pressed.connect(_on_button_pressed.bind(button))

	# 호버 피드백 (마우스/터치 호버)
	if button.mouse_entered.is_connected(_on_button_hover):
		button.mouse_entered.disconnect(_on_button_hover)

	button.mouse_entered.connect(_on_button_hover.bind(button))


func setup_input_feedback(input: LineEdit):
	# 포커스 피드백
	if input.focus_entered.is_connected(_on_input_focus):
		input.focus_entered.disconnect(_on_input_focus)

	input.focus_entered.connect(_on_input_focus.bind(input))


func _on_button_pressed(button: Button):
	apply_touch_feedback(button, TouchFeedbackType.COMBINED)


func _on_button_hover(button: Button):
	apply_touch_feedback(button, TouchFeedbackType.SCALE)


func _on_input_focus(input: LineEdit):
	apply_touch_feedback(input, TouchFeedbackType.COLOR)


func apply_touch_feedback(node: Node, feedback_type: TouchFeedbackType):
	match feedback_type:
		TouchFeedbackType.SCALE:
			apply_scale_feedback(node)
		TouchFeedbackType.COLOR:
			apply_color_feedback(node)
		TouchFeedbackType.VIBRATION:
			apply_vibration_feedback()
		TouchFeedbackType.SOUND:
			apply_sound_feedback()
		TouchFeedbackType.COMBINED:
			apply_combined_feedback(node)

	emit_signal("touch_feedback_applied", node, TouchFeedbackType.keys()[feedback_type])


func apply_scale_feedback(node: Node):
	# 기존 트윈 정리
	if feedback_tweens.has(node):
		feedback_tweens[node].kill()

	var tween = create_tween()
	feedback_tweens[node] = tween

	# 스케일 애니메이션
	tween.tween_property(node, "scale", Vector2(0.95, 0.95), 0.1)
	tween.tween_property(node, "scale", Vector2(1.0, 1.0), 0.1)

	# 트윈 완료 후 정리
	await tween.finished
	if feedback_tweens.has(node):
		feedback_tweens.erase(node)


func apply_color_feedback(node: Node):
	if not node.has_method("add_theme_color_override"):
		return

	# 기존 트윈 정리
	if feedback_tweens.has(node):
		feedback_tweens[node].kill()

	var tween = create_tween()
	feedback_tweens[node] = tween

	var original_modulate = node.modulate
	var highlight_color = Color(1.2, 1.2, 1.2, 1.0)

	# 색상 애니메이션
	tween.tween_property(node, "modulate", highlight_color, 0.15)
	tween.tween_property(node, "modulate", original_modulate, 0.15)

	await tween.finished
	if feedback_tweens.has(node):
		feedback_tweens.erase(node)


func apply_vibration_feedback():
	if OS.has_feature("mobile"):
		Input.vibrate_handheld(50)  # 50ms 진동


func apply_sound_feedback():
	# 사운드 피드백 (AudioStreamPlayer 필요)
	# 여기서는 간단한 콘솔 출력으로 대체
	print("Touch sound feedback")


func apply_combined_feedback(node: Node):
	# 복합 피드백: 스케일 + 진동
	apply_scale_feedback(node)
	apply_vibration_feedback()


# 노드 찾기 헬퍼 메서드들
func find_buttons_recursive(node: Node, buttons: Array):
	if node is Button:
		buttons.append(node)

	for child in node.get_children():
		find_buttons_recursive(child, buttons)


func find_text_nodes_recursive(node: Node, text_nodes: Array):
	if node is Label or node is Button or node is LineEdit or node is RichTextLabel:
		text_nodes.append(node)

	for child in node.get_children():
		find_text_nodes_recursive(child, text_nodes)


func find_containers_recursive(node: Node, containers: Array):
	if node is VBoxContainer or node is HBoxContainer or node is GridContainer:
		containers.append(node)

	for child in node.get_children():
		find_containers_recursive(child, containers)


func find_interactive_nodes_recursive(node: Node, interactive_nodes: Array):
	if node is Button or node is LineEdit or node is TextEdit or node is Slider:
		interactive_nodes.append(node)

	for child in node.get_children():
		find_interactive_nodes_recursive(child, interactive_nodes)


# 특정 씬별 최적화
func optimize_training_screen(scene: Node):
	# 훈련 화면 특화 최적화
	var training_cards = scene.get_tree().get_nodes_in_group("training_cards")
	for card in training_cards:
		if card.custom_minimum_size.y < 160:
			card.custom_minimum_size.y = 160


func optimize_stats_screen(scene: Node):
	# 스탯 화면 특화 최적화
	var progress_bars = []
	find_progress_bars_recursive(scene, progress_bars)

	for bar in progress_bars:
		if bar.custom_minimum_size.y < MIN_TOUCH_SIZE:
			bar.custom_minimum_size.y = MIN_TOUCH_SIZE


func find_progress_bars_recursive(node: Node, progress_bars: Array):
	if node is ProgressBar:
		progress_bars.append(node)

	for child in node.get_children():
		find_progress_bars_recursive(child, progress_bars)


# 최적화 결과 보고
func generate_optimization_report() -> Dictionary:
	var report = {"optimized_nodes_count": optimized_nodes.size(), "optimized_nodes": [], "recommendations": []}

	for node in optimized_nodes:
		report.optimized_nodes.append({"name": node.name, "type": node.get_class(), "path": node.get_path()})

	return report


# 실시간 최적화 (새로 생성된 UI 요소용)
func optimize_node_realtime(node: Node):
	if node is Button:
		optimize_button_size(node)
		setup_button_feedback(node)
	elif node is Container:
		optimize_container_spacing(node)


# 최적화 설정 변경
func set_min_touch_size(size: int):
	# 런타임에서 최소 터치 크기 변경 (const 변경 불가하므로 변수로 관리 필요)
	pass


func set_min_font_size(size: int):
	# 런타임에서 최소 폰트 크기 변경
	pass
