extends Control
## GachaCardDisplay - 가챠 카드 표시 컴포넌트
## 레어도별 연출 및 카드 정보 표시
##
## 작성일: 2025-11-26
## 참조: 04_ui_design_system.md §6.2

signal card_clicked(card_data: Dictionary)
signal flip_completed

# ============================================
# UI 노드 참조
# ============================================

@onready var card_container: Control = $CardContainer
@onready var card_back: ColorRect = $CardContainer/CardBack
@onready var card_front: Control = $CardContainer/CardFront
@onready var glow_effect: ColorRect = $CardContainer/GlowEffect
@onready var rarity_stars: Label = $CardContainer/CardFront/RarityStars
@onready var card_name: Label = $CardContainer/CardFront/CardName
@onready var card_type_label: Label = $CardContainer/CardFront/CardType
@onready var specialty_label: Label = $CardContainer/CardFront/Specialty
@onready var new_badge: Label = $CardContainer/CardFront/NewBadge

# ============================================
# 디자인 시스템 색상
# ============================================

const COLOR_BG_CARD = Color("#21262D")
const COLOR_BG_CARD_BACK = Color("#161B22")
const COLOR_TEXT_PRIMARY = Color("#E6EDF3")
const COLOR_TEXT_SECONDARY = Color("#8B949E")
const COLOR_ACCENT_PRIMARY = Color("#238636")

# 레어도별 색상
const RARITY_COLORS = {
	1: Color("#6E7681"), 2: Color("#3FB950"), 3: Color("#58A6FF"), 4: Color("#A371F7"), 5: Color("#FFD700")  # 회색  # 초록  # 파랑  # 보라  # 금색
}

# 레어도별 글로우 색상
const RARITY_GLOW = {
	1: Color(0.4, 0.4, 0.4, 0.0),
	2: Color(0.2, 0.7, 0.3, 0.2),
	3: Color(0.3, 0.5, 1.0, 0.3),
	4: Color(0.6, 0.4, 0.9, 0.4),
	5: Color(1.0, 0.8, 0.0, 0.6)
}

# ============================================
# 상태 변수
# ============================================

var _card_data: Dictionary = {}
var _is_flipped: bool = false
var _is_animating: bool = false

# ============================================
# 초기화
# ============================================


func _ready() -> void:
	# 초기 상태: 카드 뒷면 표시
	if card_front:
		card_front.visible = false
	if card_back:
		card_back.visible = true
	if glow_effect:
		glow_effect.visible = false
	if new_badge:
		new_badge.visible = false

	# 클릭 이벤트
	gui_input.connect(_on_gui_input)


# ============================================
# 공개 API
# ============================================


## 카드 데이터 설정 (뒷면 상태)
func setup(card_data: Dictionary) -> void:
	_card_data = card_data
	_is_flipped = false

	if card_front:
		card_front.visible = false
	if card_back:
		card_back.visible = true

	# 레어도에 따른 뒷면 힌트 (글로우만)
	var rarity = card_data.get("rarity", 1)
	_update_glow(rarity, false)


## 카드 즉시 표시 (애니메이션 없이)
func show_card_instant(card_data: Dictionary) -> void:
	_card_data = card_data
	_is_flipped = true
	_update_card_display()

	if card_front:
		card_front.visible = true
	if card_back:
		card_back.visible = false

	var rarity = card_data.get("rarity", 1)
	_update_glow(rarity, true)


## 카드 뒤집기 애니메이션
func flip_card() -> void:
	if _is_animating or _is_flipped:
		return

	_is_animating = true

	# 카드 데이터 표시 업데이트
	_update_card_display()

	# 뒤집기 애니메이션
	var tween = create_tween()
	tween.set_ease(Tween.EASE_IN_OUT)
	tween.set_trans(Tween.TRANS_CUBIC)

	# 1단계: 0 -> 90도 회전 (뒷면 숨김)
	tween.tween_property(card_container, "scale:x", 0.0, 0.3)
	tween.tween_callback(
		func():
			card_back.visible = false
			card_front.visible = true
	)

	# 2단계: -90 -> 0도 회전 (앞면 표시)
	tween.tween_property(card_container, "scale:x", 1.0, 0.3)
	tween.tween_callback(
		func():
			_is_flipped = true
			_is_animating = false
			_show_rarity_effect()
			flip_completed.emit()
	)


## 카드 초기화
func reset() -> void:
	_card_data = {}
	_is_flipped = false

	if card_front:
		card_front.visible = false
	if card_back:
		card_back.visible = true
	if glow_effect:
		glow_effect.visible = false
	if new_badge:
		new_badge.visible = false

	if card_container:
		card_container.scale = Vector2.ONE


## 카드 데이터 반환
func get_card_data() -> Dictionary:
	return _card_data


# ============================================
# 내부 함수
# ============================================


func _update_card_display() -> void:
 var rarity = _card_data.get("rarity", 1)
 var card_type = str(_card_data.get("type", _card_data.get("card_type", "coach"))).to_lower()
 var specialty = _card_data.get("specialty_name", "")
 var is_new = _card_data.get("is_new", false)

	# 별 표시
	if rarity_stars:
		rarity_stars.text = "★".repeat(rarity)
		rarity_stars.add_theme_color_override("font_color", RARITY_COLORS.get(rarity, Color.WHITE))

	# 카드 이름
	if card_name:
		card_name.text = _card_data.get("name", "Unknown Card")
		card_name.add_theme_color_override("font_color", COLOR_TEXT_PRIMARY)

	# 카드 타입
	if card_type_label:
		var type_names = {"manager": "감독", "coach": "코치", "tactics": "전술"}
		card_type_label.text = type_names.get(card_type, "카드")
		card_type_label.add_theme_color_override("font_color", COLOR_TEXT_SECONDARY)

	# 전문분야
	if specialty_label:
		specialty_label.text = specialty if specialty else ""
		specialty_label.add_theme_color_override("font_color", COLOR_TEXT_SECONDARY)

	# NEW 뱃지
	if new_badge:
		new_badge.visible = is_new
		if is_new:
			new_badge.text = "NEW!"
			new_badge.add_theme_color_override("font_color", COLOR_ACCENT_PRIMARY)


func _update_glow(rarity: int, is_visible: bool) -> void:
	if not glow_effect:
		return

	glow_effect.visible = is_visible and rarity >= 3
	if is_visible:
		glow_effect.color = RARITY_GLOW.get(rarity, Color.TRANSPARENT)


func _show_rarity_effect() -> void:
	"""레어도별 특수 효과"""
	var rarity = _card_data.get("rarity", 1)

	# 글로우 효과
	_update_glow(rarity, true)

	# 5성 특별 연출
	if rarity == 5:
		_animate_five_star()
	elif rarity == 4:
		_animate_four_star()
	elif rarity == 3:
		_animate_three_star()


func _animate_five_star() -> void:
	"""5성 금색 연출"""
	if not glow_effect:
		return

	var tween = create_tween()
	tween.set_loops(3)
	tween.tween_property(glow_effect, "modulate:a", 0.3, 0.2)
	tween.tween_property(glow_effect, "modulate:a", 1.0, 0.2)

	# 스케일 펄스
	var scale_tween = create_tween()
	scale_tween.tween_property(card_container, "scale", Vector2(1.1, 1.1), 0.15)
	scale_tween.tween_property(card_container, "scale", Vector2.ONE, 0.15)


func _animate_four_star() -> void:
	"""4성 보라색 연출"""
	if not glow_effect:
		return

	var tween = create_tween()
	tween.set_loops(2)
	tween.tween_property(glow_effect, "modulate:a", 0.5, 0.15)
	tween.tween_property(glow_effect, "modulate:a", 1.0, 0.15)


func _animate_three_star() -> void:
	"""3성 파란색 연출"""
	if not glow_effect:
		return

	var tween = create_tween()
	tween.tween_property(glow_effect, "modulate:a", 0.7, 0.2)
	tween.tween_property(glow_effect, "modulate:a", 1.0, 0.2)


# ============================================
# 이벤트 핸들러
# ============================================


func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		if event.pressed and event.button_index == MOUSE_BUTTON_LEFT:
			if _is_flipped:
				card_clicked.emit(_card_data)
			else:
				flip_card()
