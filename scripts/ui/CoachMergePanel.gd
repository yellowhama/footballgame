extends Control
class_name CoachMergePanel

## Phase 6.1: Coach Merge Panel UI
## 코치 카드 합성 인터페이스

signal merge_completed(result_card: Resource)
signal panel_closed

## UI References
@onready var coach_list: ItemList = %CoachList
@onready var selected_coach_label: Label = %SelectedCoachLabel
@onready var rarity_label: Label = %RarityLabel
@onready var count_label: Label = %CountLabel
@onready var result_label: Label = %ResultLabel
@onready var bonus_label: Label = %BonusLabel
@onready var merge_button: Button = %MergeButton
@onready var close_button: Button = %CloseButton
@onready var cards_container: HBoxContainer = %CardsContainer
@onready var result_container: CenterContainer = %ResultContainer
@onready var result_card_label: Label = %ResultCardLabel

## State
var _coach_card_system: Node = null
var _selected_coach_id: String = ""
var _merge_info: Dictionary = {}

## Constants
const COLOR_COMMON: Color = Color(0.7, 0.7, 0.7)
const COLOR_UNCOMMON: Color = Color(0.3, 0.8, 0.3)
const COLOR_RARE: Color = Color(0.3, 0.5, 0.9)
const COLOR_EPIC: Color = Color(0.7, 0.3, 0.9)
const COLOR_LEGENDARY: Color = Color(0.95, 0.75, 0.2)


func _ready() -> void:
	_coach_card_system = get_node_or_null("/root/CoachCardSystem")

	if close_button:
		close_button.pressed.connect(_on_close_pressed)
	if merge_button:
		merge_button.pressed.connect(_on_merge_pressed)
	if coach_list:
		coach_list.item_selected.connect(_on_coach_selected)

	result_container.visible = false
	_refresh_coach_list()


func _refresh_coach_list() -> void:
	"""보유 코치 목록 새로고침."""
	if not coach_list or not _coach_card_system:
		return

	coach_list.clear()

	# 고유 코치 ID별로 그룹화
	var coach_groups: Dictionary = {}
	var owned_coaches: Array = _coach_card_system.get_owned_coaches()

	for coach in owned_coaches:
		var coach_id: String = coach.coach_name
		if not coach_groups.has(coach_id):
			coach_groups[coach_id] = {"coach": coach, "count": 0}
		coach_groups[coach_id].count += 1

	# 리스트에 추가
	for coach_id in coach_groups:
		var group: Dictionary = coach_groups[coach_id]
		var coach: Resource = group.coach
		var count: int = group.count
		var merge_check: Dictionary = _coach_card_system.can_merge(coach_id)

		var display_text: String = "%s (%s) x%d" % [coach_id, coach.get_rarity_name(), count]

		if merge_check.can_merge:
			display_text += " [합성 가능]"

		var idx: int = coach_list.add_item(display_text)
		coach_list.set_item_metadata(idx, coach_id)

		# 등급별 색상
		var color: Color = _get_rarity_color(coach.rarity)
		coach_list.set_item_custom_fg_color(idx, color)


func _on_coach_selected(idx: int) -> void:
	"""코치 선택 시."""
	if not coach_list:
		return

	_selected_coach_id = coach_list.get_item_metadata(idx)
	_update_merge_preview()


func _update_merge_preview() -> void:
	"""합성 미리보기 업데이트."""
	if not _coach_card_system or _selected_coach_id.is_empty():
		_clear_preview()
		return

	_merge_info = _coach_card_system.get_merge_info(_selected_coach_id)

	if not _merge_info.valid:
		_clear_preview()
		return

	# 선택된 코치 정보 표시
	selected_coach_label.text = _selected_coach_id
	rarity_label.text = "등급: %s" % _merge_info.current_rarity
	count_label.text = "보유: %d / 필요: %d" % [_merge_info.owned_count, _merge_info.required_count]

	# 합성 결과 정보
	if _merge_info.result_rarity != "MAX":
		result_label.text = "결과: %s → %s" % [_merge_info.current_rarity, _merge_info.result_rarity]

		var current_bonus: float = (_merge_info.current_training_bonus - 1.0) * 100
		var result_bonus: float = (_merge_info.result_training_bonus - 1.0) * 100
		bonus_label.text = "훈련 보너스: +%.0f%% → +%.0f%%" % [current_bonus, result_bonus]
	else:
		result_label.text = "최고 등급 (합성 불가)"
		bonus_label.text = ""

	# 합성 버튼 상태
	merge_button.disabled = not _merge_info.can_merge
	if _merge_info.can_merge:
		merge_button.text = "합성하기"
	else:
		merge_button.text = "카드 부족 (%d/%d)" % [_merge_info.owned_count, _merge_info.required_count]

	_update_cards_visual()


func _update_cards_visual() -> void:
	"""카드 시각적 표시 업데이트."""
	if not cards_container:
		return

	# 기존 카드 제거
	for child in cards_container.get_children():
		child.queue_free()

	var required: int = _merge_info.required_count
	var owned: int = _merge_info.owned_count

	# 필요한 카드 수만큼 슬롯 생성
	for i in range(required):
		var card_slot: PanelContainer = PanelContainer.new()
		card_slot.custom_minimum_size = Vector2(80, 100)

		var style: StyleBoxFlat = StyleBoxFlat.new()
		style.bg_color = Color(0.15, 0.15, 0.2, 0.9)
		style.corner_radius_top_left = 8
		style.corner_radius_top_right = 8
		style.corner_radius_bottom_left = 8
		style.corner_radius_bottom_right = 8

		if i < owned:
			# 보유한 카드
			style.border_color = _get_rarity_color_by_name(_merge_info.current_rarity)
			style.border_width_left = 2
			style.border_width_right = 2
			style.border_width_top = 2
			style.border_width_bottom = 2
		else:
			# 부족한 카드
			style.border_color = Color(0.5, 0.5, 0.5, 0.5)
			style.border_width_left = 1
			style.border_width_right = 1
			style.border_width_top = 1
			style.border_width_bottom = 1

		card_slot.add_theme_stylebox_override("panel", style)

		var label: Label = Label.new()
		label.text = "?" if i >= owned else "%d" % (i + 1)
		label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
		label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		label.size_flags_vertical = Control.SIZE_EXPAND_FILL
		card_slot.add_child(label)

		cards_container.add_child(card_slot)


func _clear_preview() -> void:
	"""미리보기 초기화."""
	selected_coach_label.text = "코치를 선택하세요"
	rarity_label.text = ""
	count_label.text = ""
	result_label.text = ""
	bonus_label.text = ""
	merge_button.disabled = true
	merge_button.text = "합성하기"

	if cards_container:
		for child in cards_container.get_children():
			child.queue_free()


func _on_merge_pressed() -> void:
	"""합성 버튼 클릭."""
	if not _coach_card_system or _selected_coach_id.is_empty():
		return

	if not _merge_info.can_merge:
		return

	var result_card: Resource = _coach_card_system.merge_coaches(_selected_coach_id)

	if result_card:
		_show_merge_result(result_card)
		merge_completed.emit(result_card)
		_refresh_coach_list()
		_clear_preview()


func _show_merge_result(card: Resource) -> void:
	"""합성 결과 표시."""
	if not result_container:
		return

	result_container.visible = true
	result_card_label.text = "합성 성공!\n\n%s\n(%s)" % [card.coach_name, card.get_rarity_name()]
	result_card_label.add_theme_color_override("font_color", _get_rarity_color(card.rarity))

	# 3초 후 자동 숨김
	await get_tree().create_timer(3.0).timeout
	if result_container:
		result_container.visible = false


func _on_close_pressed() -> void:
	"""닫기 버튼 클릭."""
	panel_closed.emit()
	hide()


func _get_rarity_color(rarity: int) -> Color:
	"""등급별 색상 반환."""
	const CoachCardClass = preload("res://scripts/model/CoachCard.gd")
	match rarity:
		CoachCardClass.Rarity.COMMON:
			return COLOR_COMMON
		CoachCardClass.Rarity.UNCOMMON:
			return COLOR_UNCOMMON
		CoachCardClass.Rarity.RARE:
			return COLOR_RARE
		CoachCardClass.Rarity.EPIC:
			return COLOR_EPIC
		CoachCardClass.Rarity.LEGENDARY:
			return COLOR_LEGENDARY
		_:
			return COLOR_COMMON


func _get_rarity_color_by_name(rarity_name: String) -> Color:
	"""등급 이름으로 색상 반환."""
	match rarity_name:
		"일반":
			return COLOR_COMMON
		"고급":
			return COLOR_UNCOMMON
		"희귀":
			return COLOR_RARE
		"영웅":
			return COLOR_EPIC
		"전설":
			return COLOR_LEGENDARY
		_:
			return COLOR_COMMON


## Public API


func show_panel() -> void:
	"""패널 표시."""
	show()
	_refresh_coach_list()
	_clear_preview()


func refresh() -> void:
	"""목록 새로고침."""
	_refresh_coach_list()
