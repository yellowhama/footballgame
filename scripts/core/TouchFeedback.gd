extends Node
# 터치 피드백 유틸리티 - ART_UI_COMPLETE_GUIDE.md 준수

enum FeedbackType { LIGHT, MEDIUM, HEAVY, SUCCESS, ERROR }  # 일반 터치 (50ms)  # 성공 (100ms)  # 실패/경고 (200ms)  # 레벨업 (50ms + 100ms 조합)  # 에러 (50ms + 50ms + 50ms 조합)


static func apply_touch_feedback(button: Button, feedback_type: FeedbackType = FeedbackType.LIGHT):
	# 버튼에 터치 피드백 적용
	if not button:
		return

	# 햅틱 피드백
	trigger_haptic_feedback(feedback_type)

	# 시각적 피드백
	apply_visual_feedback(button, feedback_type)


static func trigger_haptic_feedback(feedback_type: FeedbackType):
	# 햅틱 피드백 실행
	if not OS.has_feature("mobile"):
		return

	match feedback_type:
		FeedbackType.LIGHT:
			Input.vibrate_handheld(50)
		FeedbackType.MEDIUM:
			Input.vibrate_handheld(100)
		FeedbackType.HEAVY:
			Input.vibrate_handheld(200)
		FeedbackType.SUCCESS:
			Input.vibrate_handheld(50)
			# 복합 햅틱은 간단하게 처리
			Input.vibrate_handheld(100)
		FeedbackType.ERROR:
			Input.vibrate_handheld(50)
			# 복합 햅틱은 간단하게 처리
			Input.vibrate_handheld(50)


static func apply_visual_feedback(button: Button, feedback_type: FeedbackType):
	# 시각적 피드백 적용
	if not button:
		return

	var tween = button.create_tween()

	match feedback_type:
		FeedbackType.LIGHT:
			tween.tween_property(button, "modulate", Color(0.9, 0.9, 0.9, 1.0), 0.1)
			tween.tween_property(button, "modulate", Color.WHITE, 0.1)
		FeedbackType.MEDIUM:
			tween.tween_property(button, "modulate", Color(0.8, 0.8, 0.8, 1.0), 0.1)
			tween.tween_property(button, "modulate", Color.WHITE, 0.1)
		FeedbackType.HEAVY:
			tween.tween_property(button, "modulate", Color(0.7, 0.7, 0.7, 1.0), 0.15)
			tween.tween_property(button, "modulate", Color.WHITE, 0.15)
		FeedbackType.SUCCESS:
			tween.tween_property(button, "modulate", Color(0.8, 1.0, 0.8, 1.0), 0.1)
			tween.tween_property(button, "modulate", Color.WHITE, 0.1)
		FeedbackType.ERROR:
			tween.tween_property(button, "modulate", Color(1.0, 0.8, 0.8, 1.0), 0.1)
			tween.tween_property(button, "modulate", Color.WHITE, 0.1)


static func apply_to_all_buttons(scene: Control):
	# 씬의 모든 버튼에 터치 피드백 적용
	_apply_to_buttons_recursive(scene)


static func _apply_to_buttons_recursive(node: Node):
	# 재귀적으로 모든 버튼에 터치 피드백 적용
	if node is Button:
		var button = node as Button
		button.pressed.connect(_on_button_pressed.bind(button))

	for child in node.get_children():
		_apply_to_buttons_recursive(child)


static func _on_button_pressed(button: Button):
	# 버튼 클릭 시 터치 피드백 실행
	apply_touch_feedback(button, FeedbackType.LIGHT)
