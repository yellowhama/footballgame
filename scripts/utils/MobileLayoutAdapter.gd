extends Control
class_name MobileLayoutAdapter

# 반응형 레이아웃 관리 시스템
# 화면 크기에 따라 UI 레이아웃을 자동 조정

enum ScreenSize { SMALL, MEDIUM, LARGE }  # 720x1280 이하  # 1080x1920  # 1440x2560 이상

signal screen_size_changed(new_size: ScreenSize)

var current_screen_size: ScreenSize
var target_scene: Control


func _ready():
	get_viewport().size_changed.connect(_on_viewport_size_changed)
	adapt_layout()


func setup_responsive_layout(scene: Control):
	target_scene = scene
	adapt_layout()


func _on_viewport_size_changed():
	adapt_layout()


func adapt_layout():
	if not target_scene:
		return

	var viewport_size = get_viewport().size
	var new_screen_size = determine_screen_size(viewport_size)

	if new_screen_size != current_screen_size:
		current_screen_size = new_screen_size
		emit_signal("screen_size_changed", new_screen_size)
		apply_layout_for_screen_size(new_screen_size)


func determine_screen_size(size: Vector2) -> ScreenSize:
	var min_dimension = min(size.x, size.y)

	if min_dimension <= 720:
		return ScreenSize.SMALL
	elif min_dimension <= 1080:
		return ScreenSize.MEDIUM
	else:
		return ScreenSize.LARGE


func apply_layout_for_screen_size(screen_size: ScreenSize):
	match screen_size:
		ScreenSize.SMALL:
			apply_small_layout()
		ScreenSize.MEDIUM:
			apply_medium_layout()
		ScreenSize.LARGE:
			apply_large_layout()


func apply_small_layout():
	print("Applying small screen layout")

	# 그리드 컨테이너들을 1열로 변경
	var grids = find_grid_containers()
	for grid in grids:
		grid.columns = 1

	# 폰트 크기 증가
	apply_font_scaling(1.2)

	# 간격 증가
	apply_spacing_scaling(1.3)


func apply_medium_layout():
	print("Applying medium screen layout")

	# 그리드 컨테이너들을 2열로 변경
	var grids = find_grid_containers()
	for grid in grids:
		grid.columns = 2

	# 기본 폰트 크기
	apply_font_scaling(1.0)

	# 기본 간격
	apply_spacing_scaling(1.0)


func apply_large_layout():
	print("Applying large screen layout")

	# 그리드 컨테이너들을 3열로 변경
	var grids = find_grid_containers()
	for grid in grids:
		grid.columns = 3

	# 폰트 크기 약간 감소
	apply_font_scaling(0.9)

	# 간격 약간 감소
	apply_spacing_scaling(0.9)


func find_grid_containers() -> Array:
	var grids = []
	if target_scene:
		_find_grid_containers_recursive(target_scene, grids)
	return grids


func _find_grid_containers_recursive(node: Node, grids: Array):
	if node is GridContainer:
		grids.append(node)

	for child in node.get_children():
		_find_grid_containers_recursive(child, grids)


func apply_font_scaling(scale: float):
	var base_font_size = 16
	var scaled_size = int(base_font_size * scale)

	# 모든 텍스트 노드에 폰트 크기 적용
	var text_nodes = find_text_nodes()
	for node in text_nodes:
		if node.has_method("add_theme_font_size_override"):
			node.add_theme_font_size_override("font_size", scaled_size)


func apply_spacing_scaling(scale: float):
	var base_spacing = 16
	var scaled_spacing = int(base_spacing * scale)

	# 모든 컨테이너에 간격 적용
	var containers = find_containers()
	for container in containers:
		if container.has_method("add_theme_constant_override"):
			container.add_theme_constant_override("separation", scaled_spacing)
			if container is GridContainer:
				container.add_theme_constant_override("h_separation", scaled_spacing)
				container.add_theme_constant_override("v_separation", scaled_spacing)


func find_text_nodes() -> Array:
	var text_nodes = []
	if target_scene:
		_find_text_nodes_recursive(target_scene, text_nodes)
	return text_nodes


func _find_text_nodes_recursive(node: Node, text_nodes: Array):
	if node is Label or node is Button or node is LineEdit:
		text_nodes.append(node)

	for child in node.get_children():
		_find_text_nodes_recursive(child, text_nodes)


func find_containers() -> Array:
	var containers = []
	if target_scene:
		_find_containers_recursive(target_scene, containers)
	return containers


func _find_containers_recursive(node: Node, containers: Array):
	if node is VBoxContainer or node is HBoxContainer or node is GridContainer:
		containers.append(node)

	for child in node.get_children():
		_find_containers_recursive(child, containers)


# 특정 씬별 커스텀 레이아웃 적용
func apply_training_screen_layout(screen_size: ScreenSize):
	var training_grid = target_scene.get_node_or_null("VBox/TrainingGrid")
	if training_grid and training_grid is GridContainer:
		match screen_size:
			ScreenSize.SMALL:
				training_grid.columns = 1
			ScreenSize.MEDIUM:
				training_grid.columns = 2
			ScreenSize.LARGE:
				training_grid.columns = 3


func apply_stats_screen_layout(screen_size: ScreenSize):
	var tab_container = target_scene.get_node_or_null("VBox/TabContainer")
	if tab_container:
		for i in range(tab_container.get_child_count()):
			var tab = tab_container.get_child(i)
			var grid = tab.get_node_or_null("VBox/SkillGrid")
			if grid and grid is GridContainer:
				match screen_size:
					ScreenSize.SMALL:
						grid.columns = 1
					ScreenSize.MEDIUM:
						grid.columns = 2
					ScreenSize.LARGE:
						grid.columns = 3
