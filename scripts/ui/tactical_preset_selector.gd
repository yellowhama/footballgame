extends Control
class_name TacticalPresetSelector

## Tactical Preset Selector UI Component
##
## Phase 3.3: UI 프리셋 선택 메뉴
##
## 6개 전술 프리셋 카드를 그리드로 배치하고,
## 선택을 관리하는 컴포넌트입니다.
##
## 사용법:
##   var selector = TacticalPresetSelector.new()
##   selector.preset_changed.connect(_on_preset_changed)
##   var instructions = selector.get_instructions()

## 시그널
signal preset_changed(preset_id: String)

## 선택된 프리셋
var _selected_preset_id: String = "tiki_taka"  # 기본값
var _cards: Dictionary = {}  # preset_id → TacticalPresetCard

## UI 노드 (동적 생성)
var vbox: VBoxContainer
var title_label: Label
var grid_container: GridContainer

## 타이틀 표시 여부
@export var show_title: bool = true
@export var title_text: String = "전술 선택"


func _ready():
	_create_ui()
	_create_preset_cards()
	_select_preset(_selected_preset_id)


## UI 생성
func _create_ui():
	# VBoxContainer
	vbox = VBoxContainer.new()
	vbox.set_anchors_preset(Control.PRESET_FULL_RECT)
	vbox.add_theme_constant_override("separation", 10)
	add_child(vbox)

	# Title Label (옵션)
	if show_title:
		title_label = Label.new()
		title_label.text = title_text
		title_label.add_theme_font_size_override("font_size", 16)
		title_label.add_theme_color_override("font_color", Color(1, 1, 1))
		vbox.add_child(title_label)

	# GridContainer (2x3 또는 3x2)
	grid_container = GridContainer.new()
	grid_container.columns = 3  # 3열
	grid_container.add_theme_constant_override("h_separation", 10)
	grid_container.add_theme_constant_override("v_separation", 10)
	vbox.add_child(grid_container)


## 프리셋 카드 생성
func _create_preset_cards():
	"""
	6개 프리셋 카드를 생성하고 그리드에 추가합니다.
	"""
	# 6개 프리셋 카드 생성
	for preset_id in TacticalPresets.PRESET_IDS:
		var card = TacticalPresetCard.new()
		card.set_preset(preset_id)
		card.preset_selected.connect(_on_preset_selected)
		card.custom_minimum_size = Vector2(200, 180)

		grid_container.add_child(card)
		_cards[preset_id] = card

	print("[TacticalPresetSelector] Created %d preset cards" % _cards.size())


## 프리셋 선택 이벤트
func _on_preset_selected(preset_id: String):
	"""
	카드 클릭 시 호출됩니다.

	Args:
		preset_id: 선택된 프리셋 ID
	"""
	_select_preset(preset_id)
	preset_changed.emit(preset_id)
	print("[TacticalPresetSelector] Preset changed to: %s" % preset_id)


## 프리셋 선택 (내부)
func _select_preset(preset_id: String):
	"""
	프리셋을 선택하고 UI를 업데이트합니다.

	Args:
		preset_id: 프리셋 ID
	"""
	_selected_preset_id = preset_id

	# 모든 카드 선택 해제
	for card in _cards.values():
		card.set_selected(false)

	# 선택한 카드만 하이라이트
	if _cards.has(preset_id):
		_cards[preset_id].set_selected(true)


## 선택된 프리셋 ID 반환
func get_selected_preset() -> String:
	"""
	현재 선택된 프리셋 ID 반환

	Returns:
		String: 프리셋 ID (예: "tiki_taka")
	"""
	return _selected_preset_id


## TeamInstructions 반환
func get_instructions() -> Dictionary:
	"""
	현재 선택된 프리셋의 TeamInstructions 반환

	Rust TeamInstructions 구조체와 호환되는 형식:
	{
		"tempo": "Fast",
		"pressing": "High",
		"width": "Wide",
		"build_up_play": "ShortPassing",
		"defensive_line": "High"
	}

	Returns:
		Dictionary: TeamInstructions (또는 빈 Dictionary)
	"""
	return TacticalPresets.get_instructions(_selected_preset_id)


## 프로그래밍 방식으로 프리셋 설정
func set_preset(preset_id: String):
	"""
	프로그래밍 방식으로 프리셋을 선택합니다.

	Args:
		preset_id: 프리셋 ID
	"""
	if TacticalPresets.has_preset(preset_id):
		_select_preset(preset_id)
	else:
		push_warning("[TacticalPresetSelector] Unknown preset: %s" % preset_id)


## 타이틀 텍스트 설정
func set_title(text: String):
	"""
	타이틀 텍스트를 설정합니다.

	Args:
		text: 타이틀 텍스트
	"""
	title_text = text
	if title_label:
		title_label.text = text


## 타이틀 표시/숨김
func set_title_visible(visible: bool):
	"""
	타이틀을 표시하거나 숨깁니다.

	Args:
		visible: 표시 여부
	"""
	show_title = visible
	if title_label:
		title_label.visible = visible


## 선택된 프리셋 이름 반환 (한국어)
func get_selected_preset_name() -> String:
	"""
	선택된 프리셋의 한국어 이름 반환

	Returns:
		String: 한국어 이름
	"""
	return TacticalPresets.get_name(_selected_preset_id)


## 선택된 프리셋 아이콘 반환
func get_selected_preset_icon() -> String:
	"""
	선택된 프리셋의 아이콘 반환

	Returns:
		String: 아이콘 (이모지)
	"""
	return TacticalPresets.get_icon(_selected_preset_id)


## 선택된 프리셋 색상 반환
func get_selected_preset_color() -> Color:
	"""
	선택된 프리셋의 테마 색상 반환

	Returns:
		Color: 테마 색상
	"""
	return TacticalPresets.get_color(_selected_preset_id)


## 선택된 프리셋 설명 반환
func get_selected_preset_description() -> String:
	"""
	선택된 프리셋의 설명 반환

	Returns:
		String: 설명 (한국어)
	"""
	return TacticalPresets.get_description(_selected_preset_id)


## 그리드 열 수 설정
func set_grid_columns(columns: int):
	"""
	그리드 열 수를 설정합니다.

	Args:
		columns: 열 수 (예: 2 또는 3)
	"""
	if grid_container:
		grid_container.columns = columns


## 카드 최소 크기 설정
func set_card_size(size: Vector2):
	"""
	모든 카드의 최소 크기를 설정합니다.

	Args:
		size: 최소 크기 (예: Vector2(200, 180))
	"""
	for card in _cards.values():
		card.custom_minimum_size = size
