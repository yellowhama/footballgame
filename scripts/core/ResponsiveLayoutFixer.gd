extends Node
class_name ResponsiveLayoutFixer
# 반응형 레이아웃 수정 유틸리티 - ART_UI_COMPLETE_GUIDE.md 준수


static func fix_scene_layout(node: Control):
	# 씬의 레이아웃을 반응형으로 수정
	_fix_layout_recursive(node)


static func _fix_layout_recursive(control_node: Control):
	# 재귀적으로 모든 Control 노드의 레이아웃 수정
	# 최상위 Control에 전체 화면 앵커 설정
	if control_node.get_parent() == null or control_node.get_parent() is Viewport:
		control_node.set_anchors_preset(Control.PRESET_FULL_RECT)
		control_node.offset_left = 0
		control_node.offset_top = 0
		control_node.offset_right = 0
		control_node.offset_bottom = 0

	# 고정 오프셋을 앵커 기반으로 변환
	_convert_fixed_offsets_to_anchors(control_node)

	# 터치 타겟 크기 보장
	_ensure_minimum_touch_size(control_node)

	# 자식 노드들도 재귀적으로 처리
	for child in control_node.get_children():
		if child is Control:
			_fix_layout_recursive(child)


static func _convert_fixed_offsets_to_anchors(_control: Control):
	# 고정 오프셋을 앵커 기반으로 변환
	# margin을 offset으로 변환 (이미 수동으로 변환됨)
	# 추가 변환이 필요한 경우에만 처리
	pass


static func _ensure_minimum_touch_size(control: Control):
	# 최소 터치 크기 보장 (44px)
	if control is Button or control is TextureButton:
		var current_size = control.custom_minimum_size
		if current_size.x < 44:
			current_size.x = 44
		if current_size.y < 44:
			current_size.y = 44
		control.custom_minimum_size = current_size
	elif control is Label and "button" in control.name.to_lower():
		# 버튼 역할을 하는 라벨도 44px 보장
		var current_size = control.custom_minimum_size
		if current_size.x < 44:
			current_size.x = 44
		if current_size.y < 44:
			current_size.y = 44
		control.custom_minimum_size = current_size
