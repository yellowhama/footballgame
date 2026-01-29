extends Control

class_name CharacterCreationController

signal creation_completed(character_data: Dictionary)
signal step_changed(step: int)

const QUICK_CREATION_SETTING := "football/mvp_quick_creation"

enum Steps {
	BASIC_INFO = 0, APPEARANCE = 1, ATTRIBUTE_ASSIGNMENT = 2, CATEGORY_CHOICE = 3, SPECIALTY_CHOICE = 4, CONFIRM = 5  # 신규: 외형 설정
}

@export var step_scenes: Array[PackedScene] = []

var current_step: int = Steps.BASIC_INFO
var current_scene: Control
var step_container: Control
var character_data: Dictionary = {}
var _creation_completed: bool = false  # Prevent double execution

@onready var header: Control = $VBoxContainer/Header
@onready var step_indicator: HBoxContainer = $VBoxContainer/Header/StepIndicator
@onready var content_container: PanelContainer = $VBoxContainer/ContentContainer
@onready var navigation_bar: Panel = $VBoxContainer/NavigationBar
@onready var back_button: Button = $VBoxContainer/NavigationBar/MarginContainer/HBoxContainer/BackButton
@onready var random_button: Button = $VBoxContainer/NavigationBar/MarginContainer/HBoxContainer/RandomButton
@onready var next_button: Button = $VBoxContainer/NavigationBar/MarginContainer/HBoxContainer/NextButton


func _ready() -> void:
	print("==============================================")
	print("====== CHARACTER CREATION READY START ======")
	print("==============================================")
	print("[CharacterCreation] Initializing...")
	_print_tree_structure()  # 현재 트리에서 버튼 이름/경로 확인용

	_initialize_character_data()
	_apply_theme_styles()  # ThemeManager 스타일 적용

	if _should_use_quick_creation():
		print("[CharacterCreation] MVP quick creation enabled – generating defaults.")
		call_deferred("_run_quick_create_flow")
		return

	_wire_navigation_buttons()  # 기존 Next/Back 연결
	_load_step(Steps.BASIC_INFO)
	_update_step_indicator()
	_wire_confirm_like_buttons(self)  # "생성/완료/confirm/create/finish/complete" 스캔 연결

	print("==============================================")
	print("====== CHARACTER CREATION READY END ========")
	print("==============================================")


func _apply_theme_styles() -> void:
	"""ThemeManager 스타일 적용"""
	# 배경색
	var bg = $Background
	if bg and bg is ColorRect:
		bg.color = ThemeManager.BG_PRIMARY

	# 헤더 스타일
	if header and header is Panel:
		ThemeManager.apply_header_style(header)

	# 제목 라벨
	var title = header.get_node_or_null("Title") if header else null
	if title and title is Label:
		title.add_theme_color_override("font_color", ThemeManager.TEXT_PRIMARY)
		title.add_theme_font_size_override("font_size", ThemeManager.FONT_H1)

	# 네비게이션 바 스타일
	if navigation_bar:
		ThemeManager.apply_navbar_style(navigation_bar)

	# 네비게이션 버튼 스타일
	if back_button:
		var style = ThemeManager.get_button_style("secondary")
		ThemeManager.apply_button_style(back_button, style)
	if random_button:
		var style = ThemeManager.get_button_style("warning")
		ThemeManager.apply_button_style(random_button, style)
	if next_button:
		var style = ThemeManager.get_button_style("primary")
		ThemeManager.apply_button_style(next_button, style)

	# 콘텐츠 컨테이너 스타일
	if content_container:
		var content_style = ThemeManager.create_card_style()
		content_container.add_theme_stylebox_override("panel", content_style)


func _initialize_character_data() -> void:
	# 팀 유니폼 색상 가져오기 (TeamCreation에서 설정됨)
	var team_uniform = _get_team_uniform_colors()

	character_data = {
		"basic_info": {"name": "", "position": "CM", "number": 7},
		"appearance":
		{
			# 레거시 호환 필드
			"face_preset": 0,
			"hair_style_index": 2,
			"body_type": 1,
			# 신규 Parts 기반 외형 데이터 (팀 유니폼 적용)
			"parts_appearance":
			{
				"hair_style": "medium",
				"hair_color": "brown",
				"skin_tone": "medium",
				"torso_color": team_uniform.primary,
				"sleeve_color": team_uniform.secondary
			}
		},
		"base_attributes": {},  # Step 3에서 설정됨
		"detailed_attributes": {},  # 최종 능력치 (base + bonuses)
		"selected_category": "",  # Technical/Mental/Physical
		"selected_specialties": []  # 3 specific attributes
	}


## 마이팀 유니폼 색상 조회
func _get_team_uniform_colors() -> Dictionary:
	if MyTeamData and MyTeamData.has_method("get_team_uniform"):
		var uniform = MyTeamData.get_team_uniform(true)  # 홈 유니폼
		return {"primary": uniform.get("primary", "red"), "secondary": uniform.get("secondary", "white")}
	# 기본값 (팀 미설정 시)
	return {"primary": "red", "secondary": "white"}


# 포지션 코드를 카테고리로 변환
func _position_to_category(pos: String) -> String:
	match pos:
		"ST":
			return "공격수"
		"LM", "CM", "RM":
			return "미드필더"
		"LB", "CB", "RB":
			return "수비수"
		"GK":
			return "골키퍼"
		_:
			return "미드필더"


# Step 2에서 기본 템플릿 적용
func apply_base_attributes_for_step2() -> void:
	var pos = character_data.basic_info.get("position", "CM")
	var position_category = _position_to_category(pos)
	character_data.base_attributes = AttributeTemplates.get_position_template(position_category)
	character_data.detailed_attributes = _flatten_attribute_dict(character_data.base_attributes)
	print("[CharacterCreation] Applied base attributes for position: ", pos, " (", position_category, ")")
	var ca = AttributeTemplates.calculate_ca(character_data.base_attributes)
	print("[CharacterCreation] Base CA: ", ca)


func _wire_navigation_buttons() -> void:
	print("[CharacterCreation] Wiring navigation buttons...")

	# 버튼 null 체크
	if not back_button:
		print("[CharacterCreation] ERROR: back_button is null!")
	if not random_button:
		print("[CharacterCreation] ERROR: random_button is null!")
	if not next_button:
		print("[CharacterCreation] ERROR: next_button is null!")

	if is_instance_valid(back_button):
		back_button.pressed.connect(_on_back_pressed)
		back_button.custom_minimum_size = Vector2(0, 60)
		print("[CharacterCreation] back_button connected")
	else:
		push_warning("[CharacterCreation] BackButton not found at expected path")

	if is_instance_valid(random_button):
		random_button.pressed.connect(_on_random_pressed)
		random_button.custom_minimum_size = Vector2(0, 60)
		print("[CharacterCreation] random_button connected")
	else:
		push_warning("[CharacterCreation] RandomButton not found at expected path")

	if is_instance_valid(next_button):
		next_button.pressed.connect(_on_next_pressed)  # 1~4단계에서만 사용
		next_button.custom_minimum_size = Vector2(0, 60)
		print("[CharacterCreation] next_button connected")
	else:
		push_warning("[CharacterCreation] NextButton not found at expected path")


func _wire_confirm_like_buttons(root: Node) -> void:
	# 이름/텍스트에 아래 키워드가 들어가면 '확정 버튼'으로 간주
	var name_keys = ["confirm", "create", "finish", "complete", "final"]
	for child in root.get_children():
		if child is Button:
			var nm := child.name.to_lower()
			var tx := (child as Button).text
			if "생성" in tx or "완료" in tx or _contains_any(nm, name_keys):
				if not child.pressed.is_connected(_on_confirm_pressed):
					child.pressed.connect(_on_confirm_pressed)
					print("[CharacterCreation] ✅ wired confirm-like button -> ", child.get_path())
		_wire_confirm_like_buttons(child)


func _contains_any(s: String, arr: Array) -> bool:
	for key in arr:
		if s.findn(key) != -1:
			return true
	return false


func _print_tree_structure() -> void:
	print("[CharacterCreation] Scene tree structure:")
	_print_node_recursive(self, 0)


func _print_node_recursive(node: Node, depth: int) -> void:
	var indent = ""
	for i in range(depth):
		indent += "  "
	var node_info = indent + "- " + node.name + " (" + node.get_class() + ")"
	if node is Button:
		node_info += ' [Button: "' + (node as Button).text + '"]'
	print(node_info)
	for child in node.get_children():
		_print_node_recursive(child, depth + 1)


func _on_confirm_pressed() -> void:
	print("[CharacterCreation] ========================================")
	print("[CharacterCreation] CONFIRM pressed at step=", current_step)
	var ok := _validate_current_step()
	print("[CharacterCreation] Validation result: ", ok)
	if not ok:
		print("[CharacterCreation] ❌ Validation failed, showing error")
		_show_error("현재 단계의 입력을 완료해주세요")
		return
	print("[CharacterCreation] ✅ Validation passed, calling _complete_creation()")
	_complete_creation()
	print("[CharacterCreation] 🎉 _complete_creation() finished")
	print("[CharacterCreation] ========================================")


func _load_step(step: int) -> void:
	print("[CharacterCreation] Loading step: ", step)

	# Remove current scene if exists
	if current_scene:
		current_scene.queue_free()
		current_scene = null

	# Load appropriate scene based on step
	var scene_path: String
	match step:
		Steps.BASIC_INFO:
			scene_path = "res://scenes/character_creation/Step1_BasicInfo.tscn"
		Steps.APPEARANCE:
			scene_path = "res://scenes/character_creation/Step2_Appearance.tscn"
		Steps.ATTRIBUTE_ASSIGNMENT:
			scene_path = "res://scenes/character_creation/Step2_AttributeAssignment.tscn"
			# Step 3에서는 자동으로 기본 템플릿 적용
			apply_base_attributes_for_step2()
		Steps.CATEGORY_CHOICE:
			scene_path = "res://scenes/character_creation/Step3_CategoryChoice.tscn"
		Steps.SPECIALTY_CHOICE:
			scene_path = "res://scenes/character_creation/Step4_SpecialtyChoice.tscn"
		Steps.CONFIRM:
			scene_path = "res://scenes/character_creation/Step5_Confirm.tscn"

	# Load and instantiate scene
	print("[CharacterCreation] Attempting to load scene: ", scene_path)
	var scene_resource = load(scene_path)
	if scene_resource:
		print("[CharacterCreation] Scene resource loaded successfully")
		current_scene = scene_resource.instantiate()
		print("[CharacterCreation] Scene instantiated: ", current_scene)
		content_container.add_child(current_scene)
		print("[CharacterCreation] Scene added to container")

		# Connect to step scene if it has data_updated signal
		if current_scene.has_signal("data_updated"):
			current_scene.data_updated.connect(_on_step_data_updated)
			print("[CharacterCreation] Connected data_updated signal")

		# Pass current data to the step
		if current_scene.has_method("set_character_data"):
			current_scene.set_character_data(character_data)
			print("[CharacterCreation] Character data passed to step")

		# Wire any confirm-like buttons in the loaded step
		print("[CharacterCreation] Scanning for confirm-like buttons in step scene...")
		_wire_confirm_like_buttons(current_scene)
	else:
		print("[CharacterCreation] ERROR: Failed to load scene: ", scene_path)

	# Update navigation buttons
	_update_navigation_buttons()


func _update_navigation_buttons() -> void:
	back_button.visible = current_step > Steps.BASIC_INFO
	back_button.disabled = current_step == Steps.BASIC_INFO

	# 마지막 단계에서는 NextButton 숨김 (Step5에 별도 버튼 있음)
	if current_step == Steps.CONFIRM:
		next_button.visible = false
		random_button.visible = false  # 랜덤 버튼도 숨김
	else:
		next_button.visible = true
		random_button.visible = true
		next_button.text = "다음 단계 ▶"
		next_button.modulate = Color.WHITE


func _update_step_indicator() -> void:
	var steps = step_indicator.get_children()
	for i in range(steps.size()):
		var step_label = steps[i] as Label
		if i < current_step:
			# Completed step - ThemeManager SUCCESS 색상
			step_label.add_theme_color_override("font_color", ThemeManager.SUCCESS)
			step_label.text = "✓ " + _get_step_name(i)
		elif i == current_step:
			# Current step - ThemeManager TEXT_HIGHLIGHT 색상
			step_label.add_theme_color_override("font_color", ThemeManager.TEXT_HIGHLIGHT)
			step_label.text = "● " + _get_step_name(i)
		else:
			# Future step - ThemeManager TEXT_DISABLED 색상
			step_label.add_theme_color_override("font_color", ThemeManager.TEXT_DISABLED)
			step_label.text = "○ " + _get_step_name(i)

		# 폰트 크기 적용
		step_label.add_theme_font_size_override("font_size", ThemeManager.FONT_BODY)


func _get_step_name(step: int) -> String:
	match step:
		Steps.BASIC_INFO:
			return "기본정보"
		Steps.APPEARANCE:
			return "외형"
		Steps.ATTRIBUTE_ASSIGNMENT:
			return "능력치"
		Steps.CATEGORY_CHOICE:
			return "장점선택"
		Steps.SPECIALTY_CHOICE:
			return "특장점"
		Steps.CONFIRM:
			return "확인"
		_:
			return "알 수 없음"


func _on_back_pressed() -> void:
	print("[CharacterCreation] Back button pressed")
	if current_step > Steps.BASIC_INFO:
		current_step -= 1
		_load_step(current_step)
		_update_step_indicator()
		emit_signal("step_changed", current_step)


func _on_next_pressed() -> void:
	print("[CharacterCreation] ========================================")
	print(
		"[CharacterCreation] Next button pressed! current_step = ",
		current_step,
		" (",
		_get_step_name(current_step),
		")"
	)

	# Validate current step data
	print("[CharacterCreation] Validating current step...")
	var validation_result = _validate_current_step()
	print("[CharacterCreation] Validation result: ", validation_result)

	if not validation_result:
		print("[CharacterCreation] ❌ Validation failed for step ", current_step)
		return

	print("[CharacterCreation] ✅ Validation passed")

	# NextButton은 Step 1~4에서만 사용됨 (Step 5에는 별도 버튼)
	if current_step < Steps.CONFIRM:
		print("[CharacterCreation] Moving to next step...")
		current_step += 1
		_load_step(current_step)
		_update_step_indicator()
		emit_signal("step_changed", current_step)

	print("[CharacterCreation] ========================================")


func _on_random_pressed() -> void:
	print("[CharacterCreation] Random button pressed")
	_generate_random_character()
	if current_scene and current_scene.has_method("update_display"):
		current_scene.update_display(character_data)


func _on_step_data_updated(data: Dictionary) -> void:
	print("[CharacterCreation] Step data updated: ", data)

	# Check if position changed
	var position_changed = false
	if data.has("basic_info") and data.basic_info.has("position"):
		var new_pos = data.basic_info.position
		if character_data.basic_info.get("position", "") != new_pos:
			position_changed = true
			# 포지션이 바뀌면 Step 2에서 다시 기본 템플릿을 적용할 것임
			# 여기서는 선택 사항만 초기화
			character_data.selected_category = ""
			character_data.selected_specialties = []

	# Merge step data into character_data
	for key in data:
		if (
			character_data.has(key)
			and typeof(data[key]) == TYPE_DICTIONARY
			and typeof(character_data[key]) == TYPE_DICTIONARY
		):
			# Only merge if both are dictionaries
			for subkey in data[key]:
				character_data[key][subkey] = data[key][subkey]
		else:
			# Direct assignment for non-dictionary values or new keys
			character_data[key] = data[key]

	# Apply bonuses based on current selections
	_apply_attribute_bonuses()

	if position_changed:
		print("[CharacterCreation] Position category changed, updated attributes")


func _apply_attribute_bonuses() -> void:
	# 기본 속성이 없으면 적용 (Step 2를 거치지 않은 경우)
	if not character_data.has("base_attributes") or character_data.base_attributes.is_empty():
		apply_base_attributes_for_step2()

	# 기본 템플릿에서 시작
	var final_attributes_nested = character_data.base_attributes.duplicate(true)

	# Step 3: 카테고리 보너스 적용
	if character_data.has("selected_category") and not character_data.selected_category.is_empty():
		final_attributes_nested = AttributeTemplates.apply_category_bonus(
			final_attributes_nested, character_data.selected_category
		)
		print("[CharacterCreation] Applied category bonus: ", character_data.selected_category)

	# Step 4: 특장점 보너스 적용
	if character_data.has("selected_specialties") and character_data.selected_specialties.size() > 0:
		final_attributes_nested = AttributeTemplates.apply_specialty_bonuses(
			final_attributes_nested, character_data.selected_specialties
		)
		print("[CharacterCreation] Applied specialty bonuses: ", character_data.selected_specialties)

	# 최종 능력치 저장
	character_data.detailed_attributes = _flatten_attribute_dict(final_attributes_nested)

	# CA 계산 및 출력
	var final_ca = AttributeTemplates.calculate_ca(final_attributes_nested)
	print("[CharacterCreation] Final CA: ", final_ca)


func _flatten_attribute_dict(source: Dictionary) -> Dictionary:
	var flat := {}
	for key in source.keys():
		var value = source[key]
		if value is Dictionary:
			for sub_key in value.keys():
				flat[sub_key] = value[sub_key]
		else:
			flat[key] = value
	return flat


func _get_category_attributes(category: String) -> Array:
	"""카테고리별 속성 목록 반환"""
	match category.to_lower():
		"technical":
			return [
				"corners",
				"crossing",
				"dribbling",
				"finishing",
				"first_touch",
				"free_kicks",
				"heading",
				"long_shots",
				"long_throws",
				"marking",
				"passing",
				"penalty_taking",
				"tackling",
				"technique"
			]
		"mental":
			return [
				"aggression",
				"anticipation",
				"bravery",
				"composure",
				"concentration",
				"decisions",
				"determination",
				"flair",
				"leadership",
				"off_the_ball",
				"positioning",
				"teamwork",
				"vision",
				"work_rate"
			]
		"physical":
			return ["speed", "stamina", "strength", "agility", "balance", "jumping", "natural_fitness", "acceleration"]
		"goalkeeper":
			return ["reflexes", "handling", "aerial_ability", "command_of_area", "communication", "kicking"]
		_:
			return []


func _validate_current_step() -> bool:
	print("[CharacterCreation] Validating step: ", current_step, " (", _get_step_name(current_step), ")")

	match current_step:
		Steps.BASIC_INFO:
			print("[CharacterCreation] Checking BASIC_INFO...")
			# Step1_BasicInfo의 검증 함수 사용
			if current_scene and current_scene.has_method("validate"):
				var result = current_scene.validate()
				if not result.valid:
					if current_scene.has_method("show_error"):
						current_scene.show_error(result.error)
					else:
						_show_error(result.error)
					return false
				# 검증 통과 시 데이터 업데이트
				if current_scene.has_method("get_validated_data"):
					var validated_data = current_scene.get_validated_data()
					if not validated_data.is_empty():
						_on_step_data_updated(validated_data)
			else:
				# 폴백: 기존 검증 로직
				if character_data.basic_info.name.is_empty():
					_show_error("이름을 입력해주세요")
					return false
				if character_data.basic_info.number < 1 or character_data.basic_info.number > 99:
					_show_error("등번호는 1-99 사이여야 합니다")
					return false
				if not character_data.basic_info.has("position"):
					_show_error("포지션을 선택해주세요")
					return false
			print("[CharacterCreation] BASIC_INFO validation passed")

		Steps.APPEARANCE:
			print("[CharacterCreation] APPEARANCE - auto pass (optional customization)")
			# 외형 설정은 기본값이 있으므로 항상 유효
			pass

		Steps.ATTRIBUTE_ASSIGNMENT:
			print("[CharacterCreation] ATTRIBUTE_ASSIGNMENT - auto pass")
			# Attribute assignment validation (automatic, always valid)
			pass

		Steps.CATEGORY_CHOICE:
			print("[CharacterCreation] Checking CATEGORY_CHOICE...")
			if not character_data.has("selected_category") or character_data.selected_category.is_empty():
				_show_error("장점 카테고리를 선택해주세요")
				return false
			print("[CharacterCreation] CATEGORY_CHOICE validation passed")

		Steps.SPECIALTY_CHOICE:
			print("[CharacterCreation] Checking SPECIALTY_CHOICE...")
			print("[CharacterCreation] character_data keys: ", character_data.keys())
			if character_data.has("selected_specialties"):
				print("[CharacterCreation] selected_specialties: ", character_data.selected_specialties)
			if not character_data.has("selected_specialties") or character_data.selected_specialties.size() != 3:
				_show_error("특장점 3개를 모두 선택해주세요")
				return false
			print("[CharacterCreation] SPECIALTY_CHOICE validation passed")

		Steps.CONFIRM:
			print("[CharacterCreation] CONFIRM step - no validation needed")
			# 확인 단계는 검증 필요 없음
			pass

	print("[CharacterCreation] Validation passed for step ", current_step)
	return true


func _show_error(message: String) -> void:
	# TODO: Show error dialog or toast message
	print("[CharacterCreation] Error: ", message)


func _generate_random_character() -> void:
	# Random names
	var first_names = ["김", "이", "박", "최", "정", "강", "조", "윤", "장", "임"]
	var last_names = ["민수", "준호", "성민", "지훈", "현우", "준영", "동현", "재현", "우진", "서준"]
	character_data.basic_info.name = first_names.pick_random() + last_names.pick_random()

	# Random position
	var positions = ["GK", "CB", "LB", "RB", "CDM", "CM", "CAM", "LW", "RW", "ST"]
	character_data.basic_info.position = positions.pick_random()

	# Random number
	character_data.basic_info.number = randi_range(1, 99)

	# Random appearance (레거시 호환)
	character_data.appearance.face_preset = randi_range(0, 5)
	character_data.appearance.hair_style_index = randi_range(0, 7)
	character_data.appearance.body_type = randi_range(0, 2)

	# Random Parts 기반 외형 (팀 유니폼 적용)
	var team_uniform = _get_team_uniform_colors()
	character_data.appearance.parts_appearance = PlayerAppearanceBridge.create_random_with_uniform(
		team_uniform.primary, team_uniform.secondary
	)

	# Random stats (balanced) - using detailed_attributes instead of stats
	if character_data.has("detailed_attributes"):
		var base_stat = randi_range(40, 60)
		for attr in character_data.detailed_attributes:
			character_data.detailed_attributes[attr] = base_stat + randi_range(-10, 10)


func _should_use_quick_creation() -> bool:
	for arg in OS.get_cmdline_args():
		if arg.find("GdUnitCmdTool") != -1:
			return false
	if ProjectSettings.has_setting(QUICK_CREATION_SETTING):
		return bool(ProjectSettings.get_setting(QUICK_CREATION_SETTING))
	return false  # ← MVP 모드 기본 비활성화


func _run_quick_create_flow() -> void:
	character_data.basic_info.name = "Academy Prodigy"
	character_data.basic_info.position = "ST"
	character_data.basic_info.number = 9

	character_data.appearance.face_preset = 2
	character_data.appearance.hair_style = 1
	character_data.appearance.body_type = 1

	apply_base_attributes_for_step2()
	character_data.selected_category = "Technical"
	character_data.selected_specialties = ["Finishing", "Off the Ball", "Acceleration"]

	_apply_attribute_bonuses()
	call_deferred("_complete_creation")


func _complete_creation() -> void:
	# Guard against double execution
	if _creation_completed:
		print("[CharacterCreation] ⚠️ _complete_creation() already executed, skipping duplicate call")
		return
	_creation_completed = true

	print("[CharacterCreation] 🎉 Character creation completed!")

	# position은 이제 직접 선택됨 (LB, CB, RB, LM, CM, RM, ST)
	var pos = character_data.basic_info.get("position", "CM")
	print("[CharacterCreation] Selected position: ", pos)

	print("[CharacterCreation] Step 1/7: Calculating final CA...")

	# Calculate final CA
	var final_ca = _calculate_final_ca()
	character_data["final_ca"] = final_ca
	print("[CharacterCreation] ✅ Final CA calculated: ", final_ca)

	print("[CharacterCreation] Final Character Data:")
	print("  Position: ", character_data.basic_info.get("position", "CM"))
	print("  Selected Category: ", character_data.get("selected_category", "None"))
	print("  Selected Specialties: ", character_data.get("selected_specialties", []))
	print("  Final CA: ", final_ca)

	print("[CharacterCreation] Step 2/7: Converting to OpenFootball format...")
	# Enrich detailed_attributes from CSV/template before conversion if missing or invalid
	_enrich_detailed_attributes_from_templates()
	# Convert to OpenFootball compatible format
	var openFootball_data = _convert_to_openfootball_format()
	character_data["openfootball_data"] = openFootball_data
	print("[CharacterCreation] ✅ OpenFootball conversion complete")

	print("[CharacterCreation] Step 3/7: Saving to GlobalCharacterData...")
	# Save to GlobalCharacterData - 안전한 접근
	var global_data = _get_autoload_singleton("GlobalCharacterData")
	if global_data:
		global_data.set_character_data(character_data)
		print("[CharacterCreation] ✅ Saved to GlobalCharacterData")
	else:
		print("[CharacterCreation] ⚠️ GlobalCharacterData not found or not in tree")

	print("[CharacterCreation] Step 4/7: Saving to current slot via SaveManager...")
	# Save to MVP slot by default so WeekHub can resume the loop
	var save_manager = _get_autoload_singleton("SaveManager")
	if save_manager:
		if save_manager.current_save_slot == "" or save_manager.current_save_slot == null:
			save_manager.current_save_slot = "slot_mvp"
		var current_slot = save_manager.current_save_slot
		print("[CharacterCreation] Current save slot: ", current_slot)

		# Prepare save data
		var save_data = {
			"player_name": character_data.basic_info.get("name", "Unknown"),
			"current_year": 1,
			"current_week": 1,
			"current_ability": final_ca,
			"potential_ability": 100,  # TODO: Calculate from character data
			"save_time": Time.get_datetime_string_from_system(),
			"character_data": character_data
		}

		save_manager.save_game(current_slot, save_data)
		print("[CharacterCreation] ✅ Saved to slot: ", current_slot)
	else:
		print("[CharacterCreation] ⚠️ SaveManager not found or not in tree")

	print("[CharacterCreation] Step 5/8: Saving to MyTeamManager (current session)...")
	# Save to MyTeamManager for current session - 안전한 접근
	var team_manager = _get_autoload_singleton("MyTeamManager")
	if team_manager:
		team_manager.main_character = character_data
		print("[CharacterCreation] ✅ Saved to MyTeamManager")
	else:
		print("[CharacterCreation] ⚠️ MyTeamManager not found or not in tree")

	# Refresh PlayerData autoload so new stats are used immediately
	var player_data = _get_autoload_singleton("PlayerData")
	if player_data:
		if player_data.has_method("reload_from_global"):
			player_data.reload_from_global()
			print("[CharacterCreation] ✅ PlayerData refreshed from GlobalCharacterData")

	print("[CharacterCreation] Step 6/8: Initialising MVP Week loop...")
	var date_manager = _get_autoload_singleton("DateManager")
	if date_manager and date_manager.has_method("enable_mvp_mode"):
		date_manager.enable_mvp_mode()
		print("[CharacterCreation] ✅ MVP mode enabled")

	print("[CharacterCreation] Step 7/8: Emitting creation_completed signal...")
	emit_signal("creation_completed", character_data)
	print("[CharacterCreation] ✅ Signal emitted")

	# Navigate to HomeImproved scene (육성 시작)
	print("[CharacterCreation] Step 8/8: Preparing scene transition...")
	var target_scene = "res://scenes/HomeImproved.tscn"
	print("[CharacterCreation] Target scene: ", target_scene)

	print("[CharacterCreation] Changing scene (deferred)...")
	_safe_change_scene(target_scene)


func _safe_change_scene(path: String) -> void:
	# Prefer calling through SceneTree (deferred) to avoid not-in-tree errors
	var main_loop := Engine.get_main_loop()
	if main_loop is SceneTree:
		(main_loop as SceneTree).call_deferred("change_scene_to_file", path)
		return
	if is_inside_tree() and get_tree() != null:
		get_tree().call_deferred("change_scene_to_file", path)
		return
	# Ultimate fallback: retry locally on next frame
	call_deferred("_do_change_scene_local", path)


func _do_change_scene_local(path: String) -> void:
	if is_inside_tree() and get_tree() != null:
		var result = get_tree().change_scene_to_file(path)
		print("[CharacterCreation] Scene change result (retry): ", result)
		if result != OK:
			print("[CharacterCreation] ❌ ERROR: Scene change failed on retry! Error code: ", result)
	else:
		var main_loop := Engine.get_main_loop()
		if main_loop is SceneTree:
			(main_loop as SceneTree).call_deferred("change_scene_to_file", path)


func _calculate_final_ca() -> int:
	if not character_data.has("detailed_attributes"):
		print("[CA Calculation] No detailed_attributes found, returning default 80")
		return 80

	# 디버깅: detailed_attributes 내용 확인
	print("[CA Calculation] detailed_attributes keys: ", character_data.detailed_attributes.keys())
	print("[CA Calculation] Sample attributes: ")
	for key in ["passing", "shooting", "strength", "stamina"]:
		if character_data.detailed_attributes.has(key):
			print("  ", key, ": ", character_data.detailed_attributes[key])

	# Use CAValidator for accurate calculation with position modifier
	var pos = character_data.basic_info.get("position", "CM")
	var position_category = _position_to_category(pos)
	print("[CA Calculation] Position: ", pos, " (", position_category, ")")

	var result = CAValidator.calculate_ca_openfootball_accurate(character_data.detailed_attributes, position_category)
	print("[CA Calculation] CAValidator result: ", result)

	# Perform validation check
	var validation = CAValidator.validate_ca_calculation(character_data)
	print("[CA Validation] ", validation.recommendation)
	print("[CA Validation] Godot CA: ", validation.godot_ca, " | OpenFootball CA: ", validation.openfootball_ca)
	print("[CA Validation] Position Modifier: ", "%.3f" % validation.position_modifier)

	return result.ca


## 실시간 CA 미리보기 시스템
func get_ca_preview_with_bonuses(temp_category: String = "", temp_specialties: Array = []) -> Dictionary:
	if not character_data.has("detailed_attributes"):
		return {"current_ca": 80, "projected_ca": 80, "change": 0}

	# 임시 속성 복사 (원본 변경 방지)
	var temp_attributes = character_data.detailed_attributes.duplicate(true)
	var pos = character_data.basic_info.get("position", "CM")
	var position_category = _position_to_category(pos)

	# 현재 CA 계산
	var current_result = CAValidator.calculate_ca_openfootball_accurate(temp_attributes, position_category)
	var current_ca = current_result.ca

	# 임시 카테고리 보너스 적용
	if not temp_category.is_empty():
		var category_attrs = _get_category_attributes(temp_category)
		for attr in category_attrs:
			if temp_attributes.has(attr):
				temp_attributes[attr] += 3

	# 임시 특장점 보너스 적용
	for specialty in temp_specialties:
		if temp_attributes.has(specialty):
			temp_attributes[specialty] += 3

	# 예상 CA 계산
	var projected_result = CAValidator.calculate_ca_openfootball_accurate(temp_attributes, position_category)
	var projected_ca = projected_result.ca

	return {
		"current_ca": current_ca,
		"projected_ca": projected_ca,
		"change": projected_ca - current_ca,
		"position_modifier": projected_result.position_modifier,
		"optimization_tip": CAValidator._get_optimization_tip(position_category, projected_result.position_modifier)
	}


## Step3에서 카테고리 선택 시 호출되는 미리보기
func preview_category_selection(selected_category: String) -> void:
	var preview = get_ca_preview_with_bonuses(selected_category, [])
	print("[CA Preview] Category: ", selected_category)
	print(
		"[CA Preview] Current CA: ",
		preview.current_ca,
		" → Projected CA: ",
		preview.projected_ca,
		" (+",
		preview.change,
		")"
	)
	print("[CA Preview] ", preview.optimization_tip)

	# UI 업데이트 (Step3의 CA 표시 라벨 업데이트)
	_update_ca_display_step3(preview)


## Step4에서 특장점 선택 시 호출되는 미리보기
func preview_specialty_selection(selected_specialties: Array) -> void:
	var selected_category = character_data.get("selected_category", "")
	var preview = get_ca_preview_with_bonuses(selected_category, selected_specialties)
	print("[CA Preview] Specialties: ", selected_specialties)
	print(
		"[CA Preview] Current CA: ",
		preview.current_ca,
		" → Projected CA: ",
		preview.projected_ca,
		" (+",
		preview.change,
		")"
	)
	print("[CA Preview] ", preview.optimization_tip)

	# UI 업데이트 (Step4의 CA 표시 라벨 업데이트)
	_update_ca_display_step4(preview)


## Step3 CA 표시 업데이트
func _update_ca_display_step3(preview: Dictionary) -> void:
	if current_step != Steps.CATEGORY_CHOICE:
		return

	var step3_scene = current_scene
	if not step3_scene:
		return

	# CurrentCA 라벨 업데이트
	var current_ca_label = step3_scene.get_node_or_null("ScrollContainer/VBoxContainer/Header/CADisplay/CurrentCA")
	if current_ca_label:
		current_ca_label.text = "현재 CA: " + str(preview.current_ca)

	# ProjectedCA 라벨 업데이트
	var projected_ca_label = step3_scene.get_node_or_null("ScrollContainer/VBoxContainer/Header/CADisplay/ProjectedCA")
	if projected_ca_label:
		var change_text = ""
		if preview.change > 0:
			change_text = " (+" + str(preview.change) + ")"
		projected_ca_label.text = "예상 CA: " + str(preview.projected_ca) + change_text


## Step4 CA 표시 업데이트
func _update_ca_display_step4(preview: Dictionary) -> void:
	if current_step != Steps.SPECIALTY_CHOICE:
		return

	var step4_scene = current_scene
	if not step4_scene:
		return

	# CurrentCA 라벨 업데이트
	var current_ca_label = step4_scene.get_node_or_null("ScrollContainer/VBoxContainer/Header/CADisplay/CurrentCA")
	if current_ca_label:
		current_ca_label.text = "현재 CA: " + str(preview.current_ca)

	# ProjectedCA 라벨 업데이트
	var projected_ca_label = step4_scene.get_node_or_null("ScrollContainer/VBoxContainer/Header/CADisplay/ProjectedCA")
	if projected_ca_label:
		var change_text = ""
		if preview.change > 0:
			change_text = " (+" + str(preview.change) + ")"
		projected_ca_label.text = "예상 CA: " + str(preview.projected_ca) + change_text


func _convert_to_openfootball_format() -> Dictionary:
	# Use AttributeConverter for accurate OpenFootball format conversion
	var openfootball_request = AttributeConverter.convert_character_to_openfootball_request(character_data)

	# Run conversion test for validation
	var test_result = AttributeConverter.test_full_conversion(character_data)
	print("[OpenFootball Conversion] Position: ", test_result.position_conversion.is_valid)
	print("[OpenFootball Conversion] Attributes: ", test_result.attribute_mapping.is_perfect)
	print("[OpenFootball Conversion] Success: ", test_result.success)

	if not test_result.success:
		print("[OpenFootball Conversion] Warning: Conversion issues detected!")
		for diff in test_result.attribute_mapping.differences:
			print("  - ", diff.attribute, ": ", diff.original, " → ", diff.final)

	return openfootball_request


func _get_autoload_singleton(node_name: String) -> Node:
	var main_loop := Engine.get_main_loop()
	if main_loop is SceneTree:
		return (main_loop as SceneTree).root.get_node_or_null(node_name)
	return null


# Build/repair detailed attributes using GameCache CSV (if available) or AttributeTemplates
func _enrich_detailed_attributes_from_templates() -> void:
	# If we already have a complete detailed_attributes without invalid values, keep it
	var needs_fill := false
	if not character_data.has("detailed_attributes"):
		needs_fill = true
	else:
		var da: Dictionary = character_data.detailed_attributes
		# If any attribute is missing or < 0, we will rebuild
		for k in ["passing", "stamina", "strength", "finishing", "dribbling", "acceleration", "pace", "vision"]:
			if not da.has(k) or int(da.get(k, 0)) < 0:
				needs_fill = true
				break

	if not needs_fill:
		return

	var pos := str(character_data.basic_info.get("position", "CM"))
	var pos_category := _position_to_category(pos)
	var selected_category := str(character_data.get("selected_category", ""))
	var specialties: Array = character_data.get("selected_specialties", [])

	# Try CSV cache via GameCache first (if ever exposed in future)
	var built = _build_template_attributes(pos_category, selected_category, specialties)
	character_data["detailed_attributes"] = built
	print("[CharacterCreation] Detailed attributes enriched from templates (pos=", pos_category, ")")


func _build_template_attributes(pos_category: String, category: String, specialties: Array) -> Dictionary:
	# Map Korean category to OF position code
	var of_pos := "CM"
	match pos_category:
		"골키퍼":
			of_pos = "GK"
		"수비수":
			of_pos = "CB"
		"미드필더":
			of_pos = "CM"
		"공격수":
			of_pos = "ST"
		_:
			of_pos = "CM"

	# 1) Base template by position
	var tpl := AttributeTemplates.get_position_template(pos_category)

	# 1a) If GameCache has CSV averages, merge them into template
	var gc = _get_autoload_singleton("GameCache")
	if gc and gc.has_method("get_position_average"):
		var avg: Variant = gc.get_position_average(of_pos, true)
		if typeof(avg) == TYPE_DICTIONARY and avg.has("attributes") and typeof(avg.attributes) == TYPE_DICTIONARY:
			var avg_attrs: Dictionary = avg.attributes
			for key in avg_attrs.keys():
				var k := String(key)
				# Replace in the category where the key exists
				for cat in ["technical", "mental", "physical", "goalkeeper"]:
					if cat in tpl and tpl[cat] is Dictionary and k in tpl[cat]:
						tpl[cat][k] = int(avg_attrs[k])
						break
	# 2) Apply category bonus
	if category != "":
		tpl = AttributeTemplates.apply_category_bonus(tpl, category)
	# 3) Apply specialties (normalize names to expected keys)
	var mapped_specs: Array = []
	for s in specialties:
		var n := str(s)
		# Normalize common lowercase inputs
		match n.to_lower():
			"acceleration":
				mapped_specs.append("Acceleration")
			"agility":
				mapped_specs.append("Agility")
			"speed", "pace":
				mapped_specs.append("Pace")
			"finishing":
				mapped_specs.append("Finishing")
			"passing":
				mapped_specs.append("Passing")
			"dribbling":
				mapped_specs.append("Dribbling")
			_:
				mapped_specs.append(n)
	if mapped_specs.size() > 0:
		tpl = AttributeTemplates.apply_specialty_bonuses(tpl, mapped_specs)
	# 4) Flatten to a single-level dictionary named detailed_attributes
	var out: Dictionary = {}
	for cat in ["technical", "mental", "physical", "goalkeeper"]:
		if cat in tpl and tpl[cat] is Dictionary:
			for key in tpl[cat]:
				out[key] = int(tpl[cat][key])
	return out
