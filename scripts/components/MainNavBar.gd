extends PanelContainer
## MainNavBar - 하단 네비게이션 탭바
## 대시보드, 훈련, 전술, 가챠, 선수 탭
##
## 작성일: 2025-11-26
## 참조: 03_tasks.md [4.1] 메인 네비게이션

signal tab_selected(tab_id: String)

# ============================================
# 디자인 시스템 색상
# ============================================

const COLOR_BG = Color("#0D1117")
const COLOR_BG_TAB = Color("#161B22")
const COLOR_TAB_ACTIVE = Color("#238636")
const COLOR_TAB_INACTIVE = Color("#30363D")
const COLOR_TEXT_ACTIVE = Color("#E6EDF3")
const COLOR_TEXT_INACTIVE = Color("#8B949E")
const COLOR_ICON_ACTIVE = Color("#E6EDF3")
const COLOR_ICON_INACTIVE = Color("#6E7681")

# ============================================
# 탭 정의
# ============================================

const TABS = {
	"dashboard": {"icon": "🏠", "label": "대시보드", "scene": "res://scenes/screens/DashboardScreen.tscn"},
	"training": {"icon": "💪", "label": "훈련", "scene": "res://scenes/screens/TrainingScreen.tscn"},
	"tactics": {"icon": "📋", "label": "전술", "scene": "res://scenes/screens/TacticsScreen.tscn"},
	"gacha": {"icon": "🎲", "label": "가챠", "scene": "res://scenes/screens/GachaScreen.tscn"},
	"player": {"icon": "👤", "label": "선수", "scene": "res://scenes/StatusScreenImproved.tscn"}
}

const TAB_ORDER = ["dashboard", "training", "tactics", "gacha", "player"]

# ============================================
# 상태
# ============================================

var _current_tab: String = ""
var _tab_buttons: Dictionary = {}

# ============================================
# 초기화
# ============================================


func _ready() -> void:
	_setup_style()
	_create_tabs()
	_detect_current_tab()


func _setup_style() -> void:
	var style = StyleBoxFlat.new()
	style.bg_color = COLOR_BG
	style.content_margin_top = 8
	style.content_margin_bottom = 8
	style.content_margin_left = 8
	style.content_margin_right = 8
	add_theme_stylebox_override("panel", style)

	# 고정 높이
	custom_minimum_size.y = 70


func _create_tabs() -> void:
	# 기존 자식 제거
	for child in get_children():
		child.queue_free()

	# 탭 컨테이너
	var hbox = HBoxContainer.new()
	hbox.alignment = BoxContainer.ALIGNMENT_CENTER
	hbox.add_theme_constant_override("separation", 4)
	add_child(hbox)

	for tab_id in TAB_ORDER:
		var tab_info = TABS[tab_id]
		var tab_btn = _create_tab_button(tab_id, tab_info)
		hbox.add_child(tab_btn)
		_tab_buttons[tab_id] = tab_btn


func _create_tab_button(tab_id: String, tab_info: Dictionary) -> Button:
	"""개별 탭 버튼 생성"""
	var btn = Button.new()
	btn.custom_minimum_size = Vector2(64, 54)
	btn.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	# 버튼 스타일
	var normal_style = StyleBoxFlat.new()
	normal_style.bg_color = COLOR_TAB_INACTIVE
	normal_style.corner_radius_top_left = 8
	normal_style.corner_radius_top_right = 8
	normal_style.corner_radius_bottom_left = 8
	normal_style.corner_radius_bottom_right = 8
	btn.add_theme_stylebox_override("normal", normal_style)

	var hover_style = normal_style.duplicate()
	hover_style.bg_color = COLOR_TAB_INACTIVE.lightened(0.1)
	btn.add_theme_stylebox_override("hover", hover_style)

	var pressed_style = normal_style.duplicate()
	pressed_style.bg_color = COLOR_TAB_ACTIVE
	btn.add_theme_stylebox_override("pressed", pressed_style)

	# 버튼 내용 (VBox: 아이콘 + 라벨)
	btn.text = ""  # 기본 텍스트 제거

	var vbox = VBoxContainer.new()
	vbox.alignment = BoxContainer.ALIGNMENT_CENTER
	vbox.mouse_filter = Control.MOUSE_FILTER_IGNORE
	vbox.set_anchors_preset(Control.PRESET_FULL_RECT)
	btn.add_child(vbox)

	var icon_label = Label.new()
	icon_label.name = "Icon"
	icon_label.text = tab_info.get("icon", "?")
	icon_label.add_theme_font_size_override("font_size", 20)
	icon_label.add_theme_color_override("font_color", COLOR_ICON_INACTIVE)
	icon_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	icon_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	vbox.add_child(icon_label)

	var text_label = Label.new()
	text_label.name = "Text"
	text_label.text = tab_info.get("label", "Tab")
	text_label.add_theme_font_size_override("font_size", 10)
	text_label.add_theme_color_override("font_color", COLOR_TEXT_INACTIVE)
	text_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	text_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	vbox.add_child(text_label)

	# 메타데이터 & 이벤트
	btn.set_meta("tab_id", tab_id)
	btn.pressed.connect(_on_tab_pressed.bind(tab_id))
	btn.mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND

	return btn


# ============================================
# 탭 상태 관리
# ============================================


func _detect_current_tab() -> void:
	"""현재 화면에 맞는 탭 자동 감지"""
	var current_scene = get_tree().current_scene
	if current_scene:
		var scene_file = current_scene.scene_file_path

		for tab_id in TABS.keys():
			if TABS[tab_id].get("scene", "") == scene_file:
				set_active_tab(tab_id)
				return

	# 기본값
	set_active_tab("dashboard")


func set_active_tab(tab_id: String) -> void:
	"""활성 탭 설정"""
	_current_tab = tab_id

	for id in _tab_buttons.keys():
		_update_tab_style(id, id == tab_id)


func _update_tab_style(tab_id: String, is_active: bool) -> void:
	"""탭 버튼 스타일 업데이트"""
	if not _tab_buttons.has(tab_id):
		return

	var btn = _tab_buttons[tab_id]

	# 버튼 배경 스타일
	var style = btn.get_theme_stylebox("normal") as StyleBoxFlat
	if style:
		style.bg_color = COLOR_TAB_ACTIVE if is_active else COLOR_TAB_INACTIVE

	# 아이콘/텍스트 색상
	var vbox = btn.get_child(0)
	if vbox:
		var icon_label = vbox.find_child("Icon", false, false)
		if icon_label:
			icon_label.add_theme_color_override("font_color", COLOR_ICON_ACTIVE if is_active else COLOR_ICON_INACTIVE)

		var text_label = vbox.find_child("Text", false, false)
		if text_label:
			text_label.add_theme_color_override("font_color", COLOR_TEXT_ACTIVE if is_active else COLOR_TEXT_INACTIVE)


func get_current_tab() -> String:
	return _current_tab


# ============================================
# 이벤트 핸들러
# ============================================


func _on_tab_pressed(tab_id: String) -> void:
	if tab_id == _current_tab:
		return  # 이미 활성 탭

	print("[MainNavBar] Tab selected: %s" % tab_id)
	tab_selected.emit(tab_id)

	# 화면 전환
	var tab_info = TABS.get(tab_id, {})
	var scene_path = tab_info.get("scene", "")

	if scene_path and ResourceLoader.exists(scene_path):
		get_tree().change_scene_to_file(scene_path)
	else:
		push_warning("[MainNavBar] Scene not found: %s" % scene_path)


# ============================================
# 유틸리티
# ============================================


static func create_navbar() -> PanelContainer:
	"""네비바 인스턴스 생성 (씬 없이 코드로 추가)"""
	var navbar_script = load("res://scripts/components/MainNavBar.gd")
	var navbar = PanelContainer.new()
	navbar.set_script(navbar_script)
	return navbar


static func add_to_scene(parent: Control) -> PanelContainer:
	"""부모 씬에 네비바 추가"""
	var navbar = create_navbar()
	navbar.set_anchors_preset(Control.PRESET_BOTTOM_WIDE)
	navbar.offset_top = -70
	parent.add_child(navbar)
	return navbar
