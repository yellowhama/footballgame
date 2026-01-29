extends Control

signal data_updated(data: Dictionary)

var character_data: Dictionary = {}

# 포지션별 능력치 템플릿 (CA 80 기준, 명세 V3 설계)
var position_templates: Dictionary = {
	"공격수":
	{
		# 골 결정력과 개인기 중심
		"technical":
		{
			"finishing": 60,
			"shooting": 60,
			"dribbling": 55,
			"ball_control": 55,
			"first_touch": 50,
			"technique": 50,
			"passing": 45,
			"crossing": 40,
			"heading": 45,
			"long_shots": 45,
			"free_kicks": 35,
			"penalties": 45,
			"corners": 30,
			"throw_ins": 30
		},
		"mental":
		{
			"off_the_ball": 60,
			"composure": 55,
			"anticipation": 50,
			"decisions": 50,
			"concentration": 45,
			"vision": 45,
			"flair": 50,
			"aggression": 45,
			"bravery": 45,
			"determination": 50,
			"leadership": 40,
			"teamwork": 45,
			"work_rate": 45,
			"positioning": 45
		},
		"physical":
		{
			"speed": 55,
			"acceleration": 55,
			"agility": 52,
			"balance": 52,
			"strength": 45,
			"jumping": 45,
			"stamina": 45,
			"natural_fitness": 45
		}
	},
	"미드필더":
	{
		# 패스와 시야, 활동량 중심
		"technical":
		{
			"passing": 60,
			"ball_control": 55,
			"first_touch": 55,
			"technique": 55,
			"dribbling": 50,
			"shooting": 40,
			"crossing": 50,
			"vision": 55,
			"finishing": 35,
			"heading": 40,
			"long_shots": 40,
			"free_kicks": 45,
			"corners": 45,
			"throw_ins": 40
		},
		"mental":
		{
			"vision": 60,
			"decisions": 60,
			"teamwork": 60,
			"work_rate": 60,
			"positioning": 55,
			"concentration": 55,
			"anticipation": 50,
			"leadership": 50,
			"composure": 50,
			"determination": 50,
			"off_the_ball": 45,
			"flair": 40,
			"aggression": 40,
			"bravery": 45
		},
		"physical":
		{
			"stamina": 60,
			"natural_fitness": 60,
			"agility": 52,
			"balance": 52,
			"speed": 45,
			"acceleration": 45,
			"strength": 45,
			"jumping": 40
		}
	},
	"수비수":
	{
		# 수비 위치선정과 피지컬 중심
		"technical":
		{
			"heading": 60,
			"passing": 50,
			"ball_control": 40,
			"technique": 40,
			"crossing": 35,
			"first_touch": 40,
			"dribbling": 35,
			"shooting": 25,
			"finishing": 20,
			"long_shots": 25,
			"free_kicks": 30,
			"penalties": 30,
			"corners": 25,
			"throw_ins": 45
		},
		"mental":
		{
			"positioning": 60,
			"anticipation": 60,
			"concentration": 60,
			"decisions": 55,
			"work_rate": 55,
			"teamwork": 55,
			"bravery": 60,
			"determination": 55,
			"leadership": 50,
			"aggression": 50,
			"composure": 45,
			"vision": 40,
			"off_the_ball": 35,
			"flair": 25
		},
		"physical":
		{
			"strength": 60,
			"jumping": 60,
			"stamina": 55,
			"natural_fitness": 55,
			"balance": 52,
			"agility": 45,
			"speed": 45,
			"acceleration": 45
		}
	}
}


func _ready() -> void:
	print("[Step2_AttributeAssignment] Ready")
	# Note: character_data will be passed via set_character_data() from controller
	# Don't call _assign_attributes_automatically() here as data isn't set yet


# 포지션 코드를 카테고리로 변환
func _position_to_category(pos: String) -> String:
	match pos:
		"ST", "LW", "RW", "CF":
			return "공격수"
		"LM", "CM", "RM", "CAM", "CDM", "AM", "DM":
			return "미드필더"
		"LB", "CB", "RB", "FB":
			return "수비수"
		"GK":
			return "골키퍼"
		_:
			return "미드필더"


func set_character_data(data: Dictionary) -> void:
	character_data = data
	if is_node_ready():
		_assign_attributes_automatically()


func _assign_attributes_automatically() -> void:
	print("[Step2_AttributeAssignment] 포지션별 능력치 자동 할당 시작")

	if not character_data.has("basic_info") or character_data.basic_info.is_empty():
		print("[Step2_AttributeAssignment] Waiting for basic_info from controller...")
		return

	# Derive position_category from position if not present
	var position_category = character_data.basic_info.get("position_category", "")
	if position_category.is_empty():
		var pos = character_data.basic_info.get("position", "CM")
		position_category = _position_to_category(pos)
		character_data.basic_info["position_category"] = position_category
	print("[Step2_AttributeAssignment] Position Category: ", position_category)

	# 포지션별 템플릿 가져오기
	if not position_templates.has(position_category):
		print("[Step2_AttributeAssignment] Warning: Unknown position, using 공격수")
		position_category = "공격수"

	var template = position_templates[position_category]

	# 42개 능력치 설정 (Technical + Mental + Physical + GK)
	var detailed_attributes = {}

	# Technical attributes (14개)
	for attr in template.technical:
		detailed_attributes[attr] = template.technical[attr]

	# Mental attributes (14개)
	for attr in template.mental:
		detailed_attributes[attr] = template.mental[attr]

	# Physical attributes (8개)
	for attr in template.physical:
		detailed_attributes[attr] = template.physical[attr]

	# GK attributes (6개) - 아웃필드 선수이므로 기본값
	var gk_attrs = ["reflexes", "handling", "aerial_ability", "command_of_area", "communication", "kicking"]
	for attr in gk_attrs:
		detailed_attributes[attr] = 50  # 기본값

	# character_data에 detailed_attributes 설정
	character_data["detailed_attributes"] = detailed_attributes

	# CA 계산 검증
	var calculated_ca = _calculate_ca(detailed_attributes)
	print("[Step2_AttributeAssignment] Calculated CA: ", calculated_ca)

	# 데이터 업데이트 신호 전송
	var update_data = {
		"detailed_attributes": detailed_attributes, "attribute_assignment_complete": true, "base_ca": calculated_ca
	}

	print("[Step2_AttributeAssignment] 능력치 할당 완료, 자동으로 다음 단계로 이동")
	emit_signal("data_updated", update_data)

	# 자동으로 다음 단계로 넘어가기 (1초 후)
	await get_tree().create_timer(1.0).timeout
	_proceed_to_next_step()


func _calculate_ca(attributes: Dictionary) -> int:
	# 간단한 CA 계산 (CAValidator와 동일한 공식)
	var technical_sum = 0
	var mental_sum = 0
	var physical_sum = 0
	var gk_sum = 0

	# Technical (14개)
	var technical_attrs = [
		"dribbling",
		"passing",
		"shooting",
		"crossing",
		"first_touch",
		"ball_control",
		"technique",
		"heading",
		"finishing",
		"long_shots",
		"free_kicks",
		"penalties",
		"corners",
		"throw_ins"
	]
	for attr in technical_attrs:
		if attributes.has(attr):
			technical_sum += attributes[attr]

	# Mental (14개)
	var mental_attrs = [
		"decisions",
		"concentration",
		"leadership",
		"vision",
		"teamwork",
		"work_rate",
		"positioning",
		"anticipation",
		"composure",
		"bravery",
		"determination",
		"flair",
		"off_the_ball",
		"aggression"
	]
	for attr in mental_attrs:
		if attributes.has(attr):
			mental_sum += attributes[attr]

	# Physical (8개, 2배 가중치)
	var physical_attrs = [
		"speed", "stamina", "strength", "agility", "balance", "jumping", "natural_fitness", "acceleration"
	]
	for attr in physical_attrs:
		if attributes.has(attr):
			physical_sum += attributes[attr]

	# GK (6개)
	var gk_attrs = ["reflexes", "handling", "aerial_ability", "command_of_area", "communication", "kicking"]
	for attr in gk_attrs:
		if attributes.has(attr):
			gk_sum += attributes[attr]

	# OpenFootball 공식: total_units = technical + mental + (physical × 2) + gk
	var total_units = technical_sum + mental_sum + (physical_sum * 2) + gk_sum

	# CA 계산: base_ca = (total_units - 1000) / 20
	var base_ca = (total_units - 1000) / 20.0 if total_units >= 1000 else total_units / 40.0

	return int(base_ca)


func _proceed_to_next_step() -> void:
	# CharacterCreationController를 안전하게 찾기
	var controller = _find_character_creation_controller()
	if controller and controller.has_method("_on_next_pressed"):
		print("[Step2_AttributeAssignment] 자동으로 Step3로 이동")
		controller._on_next_pressed()
	else:
		print("[Step2_AttributeAssignment] Error: Controller not found")


# CharacterCreationController를 안전하게 찾는 헬퍼 함수
func _find_character_creation_controller():
	var current_node = self
	# 최대 10 레벨까지 부모를 올라가며 찾기
	for i in range(10):
		current_node = current_node.get_parent()
		if current_node == null:
			break
		# CharacterCreationController 클래스나 스크립트를 가진 노드 찾기
		if current_node.get_script() != null:
			var script_path = current_node.get_script().resource_path
			if "CharacterCreationController" in script_path:
				return current_node
		# 또는 _on_next_pressed 메서드를 가진 노드 찾기
		if current_node.has_method("_on_next_pressed"):
			return current_node
	return null
