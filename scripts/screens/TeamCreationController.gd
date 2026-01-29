extends Control
class_name TeamCreationController

# ============================================================================
# 팀 창단 화면 컨트롤러
# UI/UX 스펙 Section 11 기반
# ============================================================================

signal team_created(team_data: Dictionary)

const CHARACTER_CREATION_SCENE = "res://scenes/CharacterCreation.tscn"
const SKELETON_CHARACTER_SCENE = "res://scenes/character/skeleton_character.tscn"

# 엠블럼 에셋 경로
const EMBLEM_PACK1_PATH = "res://assets/ui/emblems/pack1/"
const EMBLEM_PACK2_PATH = "res://assets/ui/emblems/pack2/"

# 유니폼 색상 (PlayerAppearanceBridge.UNIFORM_COLORS와 동일)
const UNIFORM_COLORS = ["red", "orange", "yellow", "green", "cyan", "blue", "purple", "pink", "white", "black", "gray"]

# 엠블럼 총 개수
const TOTAL_PACK1_EMBLEMS = 25
const TOTAL_PACK2_EMBLEMS = 18
const TOTAL_EMBLEMS = TOTAL_PACK1_EMBLEMS + TOTAL_PACK2_EMBLEMS  # 43

# UI 노드 참조
@onready var back_button: Button = %BackButton
@onready var title_label: Label = %TitleLabel

# 엠블럼 선택
@onready var emblem_display: TextureRect = %EmblemDisplay
@onready var emblem_prev_btn: Button = %EmblemPrevButton
@onready var emblem_next_btn: Button = %EmblemNextButton
@onready var emblem_label: Label = %EmblemLabel

# 팀 정보 입력
@onready var team_name_input: LineEdit = %TeamNameInput
@onready var short_name_input: LineEdit = %ShortNameInput
@onready var name_error_label: Label = %NameErrorLabel

# 유니폼 선택
@onready var uniform_tab_home: Button = %UniformTabHome
@onready var uniform_tab_away: Button = %UniformTabAway
@onready var character_preview: Node2D = %CharacterPreview
@onready var primary_color_container: HBoxContainer = %PrimaryColorContainer
@onready var secondary_color_container: HBoxContainer = %SecondaryColorContainer

# 창단 버튼
@onready var create_button: Button = %CreateButton
@onready var validation_label: Label = %ValidationLabel

# 상태 변수
var current_emblem_id: int = 0
var is_home_uniform: bool = true
var home_uniform: Dictionary = {"primary": "blue", "secondary": "white"}
var away_uniform: Dictionary = {"primary": "white", "secondary": "blue"}

# 색상 버튼 그룹
var primary_color_buttons: Array[Button] = []
var secondary_color_buttons: Array[Button] = []

# 캐릭터 프리뷰 인스턴스
var skeleton_character_instance: Node2D = null
var character_customizer: Node = null  # CharacterCustomizer는 _disabled로 이동됨


func _ready():
	print("[TeamCreation] Initializing...")

	_setup_ui()
	_connect_signals()
	_load_existing_settings()
	_update_emblem_display()
	_update_uniform_display()

	print("[TeamCreation] Ready!")


func _setup_ui():
	# 색상 버튼 생성
	_create_color_buttons(primary_color_container, true)
	_create_color_buttons(secondary_color_container, false)

	# 초기 탭 상태
	_update_uniform_tabs()

	# 캐릭터 프리뷰 생성
	_setup_character_preview()


func _setup_character_preview():
	# SkeletonCharacter 씬 로드 및 인스턴스
	var scene = load(SKELETON_CHARACTER_SCENE)
	if scene:
		skeleton_character_instance = scene.instantiate()
		skeleton_character_instance.scale = Vector2(1.5, 1.5)  # 프리뷰용 확대

		# 캐릭터 프리뷰 컨테이너에 추가
		if character_preview:
			character_preview.add_child(skeleton_character_instance)
			print("[TeamCreation] SkeletonCharacter added to preview")

			# CharacterCustomizer 찾기 (씬에 추가된 후에 찾아야 함)
			character_customizer = skeleton_character_instance.get_node_or_null("Customizer")
			if character_customizer:
				print("[TeamCreation] CharacterCustomizer found")
			else:
				print("[TeamCreation] Warning: Customizer node not found")
		else:
			print("[TeamCreation] Warning: character_preview node not found")
	else:
		print("[TeamCreation] Warning: Could not load skeleton character scene")


func _create_color_buttons(container: HBoxContainer, is_primary: bool):
	if not container:
		return

	# 기존 자식 제거
	for child in container.get_children():
		child.queue_free()

	var buttons_array = primary_color_buttons if is_primary else secondary_color_buttons
	buttons_array.clear()

	for color_name in UNIFORM_COLORS:
		var btn = Button.new()
		btn.custom_minimum_size = Vector2(40, 40)
		btn.tooltip_text = color_name.capitalize()

		# 색상 스타일 적용
		var color = _get_color_from_name(color_name)
		var style = StyleBoxFlat.new()
		style.bg_color = color
		style.corner_radius_top_left = 8
		style.corner_radius_top_right = 8
		style.corner_radius_bottom_left = 8
		style.corner_radius_bottom_right = 8
		btn.add_theme_stylebox_override("normal", style)

		# 호버 스타일
		var hover_style = style.duplicate()
		hover_style.border_width_top = 3
		hover_style.border_width_bottom = 3
		hover_style.border_width_left = 3
		hover_style.border_width_right = 3
		hover_style.border_color = Color.WHITE
		btn.add_theme_stylebox_override("hover", hover_style)
		btn.add_theme_stylebox_override("pressed", hover_style)

		# 시그널 연결
		btn.pressed.connect(func(): _on_color_selected(color_name, is_primary))

		container.add_child(btn)
		buttons_array.append(btn)


func _get_color_from_name(color_name: String) -> Color:
	match color_name:
		"red":
			return Color(0.9, 0.2, 0.2)
		"orange":
			return Color(1.0, 0.5, 0.0)
		"yellow":
			return Color(1.0, 0.9, 0.2)
		"green":
			return Color(0.2, 0.8, 0.2)
		"cyan":
			return Color(0.2, 0.8, 0.9)
		"blue":
			return Color(0.2, 0.4, 0.9)
		"purple":
			return Color(0.6, 0.2, 0.8)
		"pink":
			return Color(1.0, 0.4, 0.7)
		"white":
			return Color(0.95, 0.95, 0.95)
		"black":
			return Color(0.15, 0.15, 0.15)
		"gray":
			return Color(0.5, 0.5, 0.5)
		_:
			return Color.WHITE


func _connect_signals():
	if back_button:
		back_button.pressed.connect(_on_back_pressed)

	if emblem_prev_btn:
		emblem_prev_btn.pressed.connect(_on_emblem_prev)
	if emblem_next_btn:
		emblem_next_btn.pressed.connect(_on_emblem_next)

	if team_name_input:
		team_name_input.text_changed.connect(_on_team_name_changed)
	if short_name_input:
		short_name_input.text_changed.connect(_on_short_name_changed)
		short_name_input.max_length = 3

	if uniform_tab_home:
		uniform_tab_home.pressed.connect(func(): _select_uniform_tab(true))
	if uniform_tab_away:
		uniform_tab_away.pressed.connect(func(): _select_uniform_tab(false))

	if create_button:
		create_button.pressed.connect(_on_create_pressed)


func _load_existing_settings():
	if MyTeamData:
		var settings = MyTeamData.academy_settings

		team_name_input.text = settings.get("team_name", "Dream FC")
		short_name_input.text = settings.get("short_name", "DRM")
		current_emblem_id = settings.get("emblem_id", 0)

		var uniform = settings.get("uniform", {})
		home_uniform = uniform.get("home", {"primary": "blue", "secondary": "white"})
		away_uniform = uniform.get("away", {"primary": "white", "secondary": "blue"})


# === 엠블럼 네비게이션 ===


func _on_emblem_prev():
	current_emblem_id = (current_emblem_id - 1 + TOTAL_EMBLEMS) % TOTAL_EMBLEMS
	_update_emblem_display()


func _on_emblem_next():
	current_emblem_id = (current_emblem_id + 1) % TOTAL_EMBLEMS
	_update_emblem_display()


func _update_emblem_display():
	var texture_path: String

	if current_emblem_id < TOTAL_PACK1_EMBLEMS:
		# Pack 1: badge1.png ~ badge25.png
		texture_path = EMBLEM_PACK1_PATH + "badge%d.png" % (current_emblem_id + 1)
	else:
		# Pack 2: b1.png ~ b18.png
		var pack2_id = current_emblem_id - TOTAL_PACK1_EMBLEMS + 1
		texture_path = EMBLEM_PACK2_PATH + "b%d.png" % pack2_id

	if emblem_display:
		var texture = load(texture_path)
		if texture:
			emblem_display.texture = texture
		else:
			print("[TeamCreation] Failed to load emblem: %s" % texture_path)

	if emblem_label:
		emblem_label.text = "%d / %d" % [current_emblem_id + 1, TOTAL_EMBLEMS]


# === 팀 이름 입력 ===


func _on_team_name_changed(new_text: String):
	_validate_and_update_ui()


func _on_short_name_changed(new_text: String):
	# 자동 대문자 변환
	if short_name_input:
		short_name_input.text = new_text.to_upper()
		short_name_input.caret_column = short_name_input.text.length()
	_validate_and_update_ui()


# === 유니폼 선택 ===


func _select_uniform_tab(is_home: bool):
	is_home_uniform = is_home
	_update_uniform_tabs()
	_update_color_selection()


func _update_uniform_tabs():
	if uniform_tab_home:
		uniform_tab_home.button_pressed = is_home_uniform
	if uniform_tab_away:
		uniform_tab_away.button_pressed = not is_home_uniform


func _on_color_selected(color_name: String, is_primary: bool):
	if is_home_uniform:
		if is_primary:
			home_uniform["primary"] = color_name
		else:
			home_uniform["secondary"] = color_name
	else:
		if is_primary:
			away_uniform["primary"] = color_name
		else:
			away_uniform["secondary"] = color_name

	_update_uniform_display()
	_validate_and_update_ui()


func _update_uniform_display():
	_update_color_selection()
	_update_character_preview()


func _update_color_selection():
	var current_uniform = home_uniform if is_home_uniform else away_uniform

	# 주색상 버튼 강조
	for i in range(primary_color_buttons.size()):
		var btn = primary_color_buttons[i]
		var color_name = UNIFORM_COLORS[i]
		_set_button_selected(btn, color_name == current_uniform.get("primary", ""))

	# 보조색상 버튼 강조
	for i in range(secondary_color_buttons.size()):
		var btn = secondary_color_buttons[i]
		var color_name = UNIFORM_COLORS[i]
		_set_button_selected(btn, color_name == current_uniform.get("secondary", ""))


func _set_button_selected(btn: Button, selected: bool):
	if not btn:
		return

	# 선택된 버튼에 테두리 추가
	var normal_style = btn.get_theme_stylebox("normal")
	if normal_style is StyleBoxFlat:
		var style = normal_style.duplicate() as StyleBoxFlat
		if selected:
			style.border_width_top = 4
			style.border_width_bottom = 4
			style.border_width_left = 4
			style.border_width_right = 4
			style.border_color = Color.YELLOW
		else:
			style.border_width_top = 0
			style.border_width_bottom = 0
			style.border_width_left = 0
			style.border_width_right = 0
		btn.add_theme_stylebox_override("normal", style)


func _update_character_preview():
	var current_uniform = home_uniform if is_home_uniform else away_uniform

	if character_customizer:
		var primary = current_uniform.get("primary", "blue")
		var secondary = current_uniform.get("secondary", "white")
		character_customizer.set_uniform_colors(primary, secondary)
		print("[TeamCreation] Preview uniform applied: %s / %s" % [primary, secondary])
	else:
		print("[TeamCreation] Warning: character_customizer not available")


# === 유효성 검사 ===


func _validate_and_update_ui():
	var team_name = team_name_input.text if team_name_input else ""
	var short_name = short_name_input.text if short_name_input else ""

	var errors: Array = []

	# 팀 이름 검사
	if team_name.length() < 2:
		errors.append("팀 이름은 2자 이상")
	elif team_name.length() > 20:
		errors.append("팀 이름은 20자 이하")

	# 약어 검사
	if short_name.length() != 3:
		errors.append("약어는 3자")

	# 유니폼 충돌 검사
	if home_uniform.get("primary", "") == away_uniform.get("primary", ""):
		errors.append("홈/원정 주색상이 같음")

	# UI 업데이트
	if validation_label:
		if errors.is_empty():
			validation_label.text = ""
			validation_label.visible = false
		else:
			validation_label.text = " | ".join(errors)
			validation_label.visible = true
			validation_label.add_theme_color_override("font_color", Color(1, 0.3, 0.3))

	if create_button:
		create_button.disabled = not errors.is_empty()

	return errors.is_empty()


# === 버튼 액션 ===


func _on_back_pressed():
	# 이전 화면으로 돌아가기
	get_tree().change_scene_to_file("res://scenes/CareerIntroScreen.tscn")


func _on_create_pressed():
	if not _validate_and_update_ui():
		return

	# 팀 데이터 저장
	var team_data = {
		"team_name": team_name_input.text,
		"short_name": short_name_input.text.to_upper(),
		"emblem_id": current_emblem_id,
		"uniform": {"home": home_uniform.duplicate(), "away": away_uniform.duplicate()}
	}

	# MyTeamData에 저장
	if MyTeamData:
		MyTeamData.save_academy_settings(team_data)

	print("[TeamCreation] Team created: %s (%s)" % [team_data["team_name"], team_data["short_name"]])

	# 시그널 발송
	team_created.emit(team_data)

	# 캐릭터 생성 화면으로 이동
	_transition_to_character_creation()


func _transition_to_character_creation():
	var tween = get_tree().create_tween()
	tween.tween_property(self, "modulate:a", 0.0, 0.3)
	tween.tween_callback(func(): get_tree().change_scene_to_file(CHARACTER_CREATION_SCENE))
