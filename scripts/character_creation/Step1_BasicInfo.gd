extends Control

signal data_updated(data: Dictionary)
signal validation_failed(error: String)

const MIN_NAME_LENGTH = 2
const MAX_NAME_LENGTH = 20
const MIN_NUMBER = 1
const MAX_NUMBER = 99

var character_data: Dictionary = {}
var selected_position: String = "CM"
var current_number: int = 7
var _error_label: Label = null

@onready var name_input: LineEdit = $ScrollContainer/VBoxContainer/NameSection/HBoxContainer/NameInput
@onready var random_name_button: Button = $ScrollContainer/VBoxContainer/NameSection/HBoxContainer/RandomNameButton
@onready var number_display: Label = $ScrollContainer/VBoxContainer/NumberSection/HBoxContainer/NumberDisplay
@onready var decrease_button: Button = $ScrollContainer/VBoxContainer/NumberSection/HBoxContainer/DecreaseButton
@onready var increase_button: Button = $ScrollContainer/VBoxContainer/NumberSection/HBoxContainer/IncreaseButton

# Position system - 7 specific positions
var position_buttons: Dictionary = {}
var position_info: Dictionary = {
	"ST": {"title": "⚽ ST (스트라이커)", "description": "최전방에서 골을 넣는 공격수", "features": ["마무리", "위치선정", "헤딩", "스피드"]},
	"LM": {"title": "🏃 LM (왼쪽 미드필더)", "description": "왼쪽 측면에서 공수를 담당", "features": ["크로스", "드리블", "스피드", "활동량"]},
	"CM": {"title": "🎯 CM (중앙 미드필더)", "description": "중앙에서 경기를 조율하는 핵심", "features": ["패스", "시야", "체력", "볼배급"]},
	"RM": {"title": "🏃 RM (오른쪽 미드필더)", "description": "오른쪽 측면에서 공수를 담당", "features": ["크로스", "드리블", "스피드", "활동량"]},
	"LB": {"title": "🛡️ LB (왼쪽 풀백)", "description": "왼쪽 측면 수비수", "features": ["태클", "스피드", "크로스", "체력"]},
	"CB": {"title": "🛡️ CB (센터백)", "description": "중앙에서 골대를 지키는 수비수", "features": ["태클", "헤딩", "위치선정", "리더십"]},
	"RB": {"title": "🛡️ RB (오른쪽 풀백)", "description": "오른쪽 측면 수비수", "features": ["태클", "스피드", "크로스", "체력"]}
}


func _ready() -> void:
	print("[Step1_BasicInfo] Ready")
	_setup_controls()
	_connect_signals()
	_create_error_label()


func _setup_controls() -> void:
	# 7개 포지션 버튼 동적 생성
	_create_position_buttons()

	# 포지션 정보 표시를 위한 컨테이너 생성
	_create_position_info_display()

	# Set initial values from character_data if available
	if character_data.has("basic_info"):
		if character_data.basic_info.has("name"):
			name_input.text = character_data.basic_info.name
		if character_data.basic_info.has("number"):
			current_number = character_data.basic_info.number
			number_display.text = str(current_number)
		if character_data.basic_info.has("position"):
			selected_position = character_data.basic_info.position

	# 포지션 선택 상태 업데이트 및 정보 표시
	_update_position_selection()
	_update_position_info_display()


func _create_position_buttons() -> void:
	# 기존 PositionSection 찾기 또는 생성
	var scroll_vbox = $ScrollContainer/VBoxContainer
	var position_section = scroll_vbox.get_node_or_null("PositionSection")

	if position_section:
		# 기존 버튼들 제거
		for child in position_section.get_children():
			child.queue_free()
	else:
		# PositionSection 생성
		position_section = VBoxContainer.new()
		position_section.name = "PositionSection"
		scroll_vbox.add_child(position_section)
		scroll_vbox.move_child(position_section, 2)  # NumberSection 다음에 배치

	# 타이틀 라벨
	var title_label = Label.new()
	title_label.text = "포지션 선택"
	title_label.add_theme_font_size_override("font_size", 24)
	title_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	position_section.add_child(title_label)

	# 포지션 버튼 컨테이너 (2행으로 배치)
	var grid = GridContainer.new()
	grid.columns = 4
	grid.add_theme_constant_override("h_separation", 10)
	grid.add_theme_constant_override("v_separation", 10)
	position_section.add_child(grid)

	# 포지션 순서: ST, LM, CM, RM, LB, CB, RB (+ 빈칸)
	var positions_order = ["", "ST", "", "", "LM", "CM", "RM", "", "LB", "CB", "RB", ""]

	for pos in positions_order:
		if pos == "":
			# 빈 공간
			var spacer = Control.new()
			spacer.custom_minimum_size = Vector2(80, 50)
			grid.add_child(spacer)
		else:
			var btn = Button.new()
			btn.text = pos
			btn.custom_minimum_size = Vector2(80, 50)
			btn.toggle_mode = true
			btn.pressed.connect(_on_position_button_pressed.bind(pos))
			grid.add_child(btn)
			position_buttons[pos] = btn


func _connect_signals() -> void:
	# Name input
	name_input.text_changed.connect(_on_name_changed)
	random_name_button.pressed.connect(_on_random_name_pressed)

	# Number controls
	decrease_button.pressed.connect(_on_decrease_number)
	increase_button.pressed.connect(_on_increase_number)
	# Position buttons are connected in _create_position_buttons()


func set_character_data(data: Dictionary) -> void:
	character_data = data
	if is_node_ready():
		_setup_controls()


func update_display(data: Dictionary) -> void:
	character_data = data
	_setup_controls()


func _on_name_changed(new_text: String) -> void:
	_emit_data_update()


func _on_random_name_pressed() -> void:
	var first_names = ["김", "이", "박", "최", "정", "강", "조", "윤", "장", "임"]
	var last_names = ["민수", "준호", "성민", "지훈", "현우", "준영", "동현", "재현", "우진", "서준"]
	var random_name = first_names.pick_random() + last_names.pick_random()
	name_input.text = random_name
	_emit_data_update()


func _on_position_button_pressed(pos: String) -> void:
	# 포지션 설정
	selected_position = pos

	_update_position_selection()
	_update_position_info_display()
	_emit_data_update()


func _on_decrease_number() -> void:
	if current_number > 1:
		current_number -= 1
		number_display.text = str(current_number)
		_emit_data_update()


func _on_increase_number() -> void:
	if current_number < 99:
		current_number += 1
		number_display.text = str(current_number)
		_emit_data_update()


func _update_position_selection() -> void:
	# Update button pressed states based on selected position
	for pos in position_buttons:
		var btn = position_buttons[pos]
		btn.button_pressed = (pos == selected_position)


func _create_position_info_display() -> void:
	# 포지션 정보 표시 컨테이너 생성
	var info_container = VBoxContainer.new()
	info_container.name = "PositionInfoSection"
	info_container.add_theme_constant_override("separation", 10)

	# 구분선 추가
	var separator = HSeparator.new()
	separator.add_theme_constant_override("separation", 20)
	info_container.add_child(separator)

	var title_label = Label.new()
	title_label.name = "TitleLabel"
	title_label.text = "⚽ 공격수"
	title_label.add_theme_font_size_override("font_size", 28)
	title_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER

	var desc_label = Label.new()
	desc_label.name = "DescriptionLabel"
	desc_label.text = "골을 넣는 것이 주 임무입니다"
	desc_label.add_theme_font_size_override("font_size", 20)
	desc_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER

	var features_label = Label.new()
	features_label.name = "FeaturesLabel"
	features_label.text = "특징: 골키퍼 앞 마무리 • 드리블 돌파 • 개인 플레이 • 스피드"
	features_label.add_theme_font_size_override("font_size", 18)
	features_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	features_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART

	var future_label = Label.new()
	future_label.name = "FuturePositionsLabel"
	future_label.text = "🎯 세부 포지션: ST(스트라이커), CF(중앙공격수), LW/RW(윙어)"
	future_label.add_theme_font_size_override("font_size", 18)
	future_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	future_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART

	info_container.add_child(title_label)
	info_container.add_child(desc_label)
	info_container.add_child(features_label)
	info_container.add_child(future_label)

	$ScrollContainer/VBoxContainer.add_child(info_container)


func _update_position_info_display() -> void:
	# Update position information display
	if not position_info.has(selected_position):
		return

	# Check if info container exists
	var info_container = $ScrollContainer/VBoxContainer.get_node_or_null("PositionInfoSection")
	if not info_container:
		print("[Step1_BasicInfo] Warning: PositionInfoSection not found")
		return

	var info = position_info[selected_position]

	# Update title
	var title_node = info_container.get_node_or_null("TitleLabel")
	if title_node:
		title_node.text = info.title

	# Update description
	var desc_node = info_container.get_node_or_null("DescriptionLabel")
	if desc_node:
		desc_node.text = info.description

	# Update features
	var features_text = "특징: " + " • ".join(info.features)
	var features_node = info_container.get_node_or_null("FeaturesLabel")
	if features_node:
		features_node.text = features_text

	# FuturePositionsLabel은 더 이상 필요 없음 (구체 포지션 직접 선택)
	var future_node = info_container.get_node_or_null("FuturePositionsLabel")
	if future_node:
		future_node.visible = false


func _emit_data_update() -> void:
	var data = {"basic_info": {"name": name_input.text, "position": selected_position, "number": current_number}}
	emit_signal("data_updated", data)


## 에러 라벨 생성
func _create_error_label() -> void:
	_error_label = Label.new()
	_error_label.name = "ErrorLabel"
	_error_label.add_theme_color_override("font_color", Color.RED)
	_error_label.add_theme_font_size_override("font_size", 16)
	_error_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	_error_label.visible = false

	# NameSection 아래에 추가
	var name_section = $ScrollContainer/VBoxContainer/NameSection
	if name_section:
		var parent = name_section.get_parent()
		var idx = name_section.get_index()
		parent.add_child(_error_label)
		parent.move_child(_error_label, idx + 1)


## 이름 검증
func validate_name() -> Dictionary:
	var name_text = name_input.text.strip_edges()

	if name_text.is_empty():
		return {"valid": false, "error": "이름을 입력해주세요"}

	if name_text.length() < MIN_NAME_LENGTH:
		return {"valid": false, "error": "이름은 최소 %d자 이상이어야 합니다" % MIN_NAME_LENGTH}

	if name_text.length() > MAX_NAME_LENGTH:
		return {"valid": false, "error": "이름은 최대 %d자까지 가능합니다" % MAX_NAME_LENGTH}

	# 특수문자 체크 (한글, 영문, 숫자, 공백만 허용)
	var regex = RegEx.new()
	regex.compile("^[가-힣a-zA-Z0-9\\s]+$")
	if not regex.search(name_text):
		return {"valid": false, "error": "이름에 특수문자를 사용할 수 없습니다"}

	return {"valid": true, "error": ""}


## 번호 검증
func validate_number() -> Dictionary:
	if current_number < MIN_NUMBER or current_number > MAX_NUMBER:
		return {"valid": false, "error": "번호는 %d~%d 사이여야 합니다" % [MIN_NUMBER, MAX_NUMBER]}

	return {"valid": true, "error": ""}


## 전체 검증
func validate() -> Dictionary:
	var name_result = validate_name()
	if not name_result.valid:
		return name_result

	var number_result = validate_number()
	if not number_result.valid:
		return number_result

	return {"valid": true, "error": ""}


## 에러 표시
func show_error(message: String) -> void:
	if _error_label:
		_error_label.text = message
		_error_label.visible = true
		validation_failed.emit(message)

		# 3초 후 자동 숨김
		await get_tree().create_timer(3.0).timeout
		if _error_label:
			_error_label.visible = false


## 에러 숨김
func hide_error() -> void:
	if _error_label:
		_error_label.visible = false


## 검증 후 데이터 반환 (다음 단계 진행 시 사용)
func get_validated_data() -> Dictionary:
	var result = validate()
	if not result.valid:
		show_error(result.error)
		return {}

	hide_error()
	return {
		"basic_info": {"name": name_input.text.strip_edges(), "position": selected_position, "number": current_number}
	}
