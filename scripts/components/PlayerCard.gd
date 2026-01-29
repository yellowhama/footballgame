extends PanelContainer
class_name PlayerCard
## 표준화된 선수 카드 컴포넌트
## Phase 2: UI_UX_Design_Specification.md 기반 표준화
## ThemeManager 스타일 시스템 사용

signal selected(player_data: Dictionary)
signal double_clicked(player_data: Dictionary)

# 카드 데이터
var _player_data: Dictionary = {}
var _is_selected: bool = false

# UI 노드들 (동적 생성)
var _hbox: HBoxContainer
var _info_vbox: VBoxContainer
var _name_label: Label
var _position_label: Label
var _overall_label: Label
var _stars_label: Label

# 설정
@export var show_stars: bool = true
@export var compact_mode: bool = false
@export var show_overall: bool = true


func _ready():
	_create_ui()
	_apply_theme_style()
	_setup_interaction()


func _create_ui():
	"""UI 구조 동적 생성"""
	custom_minimum_size = Vector2(0, 80 if compact_mode else 100)
	size_flags_horizontal = Control.SIZE_EXPAND_FILL

	# 메인 HBox
	_hbox = HBoxContainer.new()
	_hbox.add_theme_constant_override("separation", ThemeManager.SPACE_MD)
	add_child(_hbox)

	# 정보 VBox
	_info_vbox = VBoxContainer.new()
	_info_vbox.add_theme_constant_override("separation", ThemeManager.SPACE_XS)
	_info_vbox.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_hbox.add_child(_info_vbox)

	# 상단: 이름 + 별 등급
	var top_hbox = HBoxContainer.new()
	top_hbox.add_theme_constant_override("separation", ThemeManager.SPACE_SM)
	_info_vbox.add_child(top_hbox)

	_name_label = Label.new()
	_name_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_name_label.add_theme_font_size_override("font_size", ThemeManager.FONT_H3)
	_name_label.add_theme_color_override("font_color", ThemeManager.TEXT_PRIMARY)
	top_hbox.add_child(_name_label)

	if show_stars:
		_stars_label = Label.new()
		_stars_label.add_theme_font_size_override("font_size", ThemeManager.FONT_CAPTION)
		_stars_label.add_theme_color_override("font_color", ThemeManager.TEXT_HIGHLIGHT)
		top_hbox.add_child(_stars_label)

	# 하단: 포지션 + OVR
	var bottom_hbox = HBoxContainer.new()
	bottom_hbox.add_theme_constant_override("separation", ThemeManager.SPACE_SM)
	_info_vbox.add_child(bottom_hbox)

	_position_label = Label.new()
	_position_label.add_theme_font_size_override("font_size", ThemeManager.FONT_BODY)
	bottom_hbox.add_child(_position_label)

	if show_overall:
		_overall_label = Label.new()
		_overall_label.add_theme_font_size_override("font_size", ThemeManager.FONT_BODY)
		_overall_label.add_theme_color_override("font_color", ThemeManager.TEXT_SECONDARY)
		bottom_hbox.add_child(_overall_label)


func _apply_theme_style():
	"""ThemeManager 스타일 적용"""
	var position = _player_data.get("position", "")
	ThemeManager.apply_player_card_style(self, position, _is_selected)


func _setup_interaction():
	"""상호작용 설정"""
	mouse_entered.connect(_on_mouse_entered)
	mouse_exited.connect(_on_mouse_exited)
	gui_input.connect(_on_gui_input)


func setup(data: Dictionary):
	"""선수 데이터로 카드 초기화"""
	_player_data = data
	_update_display()
	_apply_theme_style()


func _update_display():
	"""UI 업데이트"""
	var name = _player_data.get("name", "Unknown")
	var position = _player_data.get("position", "")
	var overall = _player_data.get("overall", 0)

	if _name_label:
		_name_label.text = name

	if _position_label:
		_position_label.text = position
		_position_label.add_theme_color_override("font_color", ThemeManager.get_position_color(position))

	if _overall_label:
		_overall_label.text = "OVR %d" % overall
		_overall_label.add_theme_color_override("font_color", ThemeManager.get_stat_color(overall))

	if _stars_label and show_stars:
		_stars_label.text = ThemeManager.get_star_rating(overall)


func set_selected(selected: bool):
	"""선택 상태 설정"""
	_is_selected = selected
	_apply_theme_style()


func is_selected() -> bool:
	return _is_selected


func get_player_data() -> Dictionary:
	return _player_data


func _on_mouse_entered():
	"""호버 상태"""
	if not _is_selected:
		var position = _player_data.get("position", "")
		var hover_style = ThemeManager.create_player_card_hover_style(position)
		add_theme_stylebox_override("panel", hover_style)

	# 스케일 애니메이션
	var tween = create_tween()
	tween.tween_property(self, "scale", Vector2(1.02, 1.02), 0.1)


func _on_mouse_exited():
	"""호버 해제"""
	_apply_theme_style()

	# 스케일 복원
	var tween = create_tween()
	tween.tween_property(self, "scale", Vector2(1.0, 1.0), 0.1)


func _on_gui_input(event: InputEvent):
	"""입력 처리"""
	if event is InputEventMouseButton and event.pressed:
		if event.button_index == MOUSE_BUTTON_LEFT:
			if event.double_click:
				double_clicked.emit(_player_data)
			else:
				selected.emit(_player_data)


# ============================================================================
# 정적 팩토리 메서드 (편의 함수)
# ============================================================================


static func create_from_data(data: Dictionary, compact: bool = false) -> PlayerCard:
	"""선수 데이터로 카드 생성"""
	var card = PlayerCard.new()
	card.compact_mode = compact
	card.setup(data)
	return card


static func create_list(players: Array, compact: bool = false) -> Array[PlayerCard]:
	"""여러 선수 데이터로 카드 리스트 생성"""
	var cards: Array[PlayerCard] = []
	for player in players:
		if player is Dictionary:
			cards.append(create_from_data(player, compact))
	return cards
