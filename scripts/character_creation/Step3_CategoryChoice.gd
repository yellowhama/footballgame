extends Control

signal data_updated(data: Dictionary)

var character_data: Dictionary = {}
var selected_category: String = ""
var current_ca: int = 80
var projected_ca: int = 80

@onready var category_buttons: Dictionary = {}
@onready var info_display: Dictionary = {}

# Category information
var category_info: Dictionary = {
	"Technical":
	{
		"title": "⚙️ Technical",
		"subtitle": "기술적 완성도",
		"description": "모든 기술 능력치에 +3 보너스",
		"bonus_text": "+3 모든 기술",
		"attributes":
		[
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
		],
		"features": ["슛 정확도", "드리블", "볼 터치", "기술적 플레이"],
		"ca_bonus": 12
	},
	"Mental":
	{
		"title": "🧠 Mental",
		"subtitle": "게임 이해력",
		"description": "모든 정신력 능력치에 +3 보너스",
		"bonus_text": "+3 모든 정신력",
		"attributes":
		[
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
		],
		"features": ["위치선정", "판단력", "집중력", "게임 이해"],
		"ca_bonus": 12
	},
	"Physical":
	{
		"title": "💪 Physical",
		"subtitle": "신체 능력",
		"description": "모든 피지컬 능력치에 +3 보너스",
		"bonus_text": "+3 모든 피지컬",
		"attributes":
		["speed", "stamina", "strength", "agility", "balance", "jumping", "natural_fitness", "acceleration"],
		"features": ["스피드", "파워", "지구력", "민첩성"],
		"ca_bonus": 15  # Physical has *2 multiplier in CA calculation
	}
}


func _ready() -> void:
	print("[Step3_CategoryChoice] Ready")
	_setup_controls()
	_connect_signals()


func _setup_controls() -> void:
	# Setup category buttons
	var button_container = $ScrollContainer/VBoxContainer/CategorySelection
	category_buttons["Technical"] = button_container.get_node("TechnicalButton")
	category_buttons["Mental"] = button_container.get_node("MentalButton")
	category_buttons["Physical"] = button_container.get_node("PhysicalButton")

	# Setup info display elements with null checks
	var current_ca_label = $ScrollContainer/VBoxContainer/Header/CADisplay/CurrentCA
	var projected_ca_label = $ScrollContainer/VBoxContainer/Header/CADisplay/ProjectedCA
	var category_title_label = $ScrollContainer/VBoxContainer/CategoryInfo/TitleLabel
	var category_desc_label = $ScrollContainer/VBoxContainer/CategoryInfo/DescriptionLabel
	var features_list_label = $ScrollContainer/VBoxContainer/CategoryInfo/FeaturesLabel

	if current_ca_label:
		info_display["current_ca"] = current_ca_label
	else:
		print("[Step3_CategoryChoice] Warning: CurrentCA label not found")

	if projected_ca_label:
		info_display["projected_ca"] = projected_ca_label
	else:
		print("[Step3_CategoryChoice] Warning: ProjectedCA label not found")

	if category_title_label:
		info_display["category_title"] = category_title_label
	else:
		print("[Step3_CategoryChoice] Warning: CategoryInfo TitleLabel not found")

	if category_desc_label:
		info_display["category_description"] = category_desc_label
	else:
		print("[Step3_CategoryChoice] Warning: CategoryInfo DescriptionLabel not found")

	if features_list_label:
		info_display["features_list"] = features_list_label
	else:
		print("[Step3_CategoryChoice] Warning: CategoryInfo FeaturesLabel not found")

	# Set initial values from character_data if available
	if character_data.has("basic_info") and character_data.basic_info.has("position_category"):
		var position_text = "선택한 포지션: " + character_data.basic_info.position_category
		# Update subtitle to include position info
		var subtitle_label = $ScrollContainer/VBoxContainer/Header/SubtitleLabel
		if subtitle_label:
			subtitle_label.text = position_text + "\n어떤 분야에서 뛰어난 선수가 되고 싶나요?"
		else:
			print("[Step3_CategoryChoice] Warning: SubtitleLabel not found")

	_update_ca_display()


func _connect_signals() -> void:
	# Category buttons
	for category in category_buttons:
		var btn = category_buttons[category]
		btn.pressed.connect(_on_category_selected.bind(category))


func set_character_data(data: Dictionary) -> void:
	character_data = data
	current_ca = _calculate_current_ca()
	if is_node_ready():
		_setup_controls()


func update_display(data: Dictionary) -> void:
	character_data = data
	current_ca = _calculate_current_ca()
	_setup_controls()


func _on_category_selected(category: String) -> void:
	selected_category = category
	_update_category_selection()
	_update_category_info_display()
	_update_ca_display()

	# 실시간 CA 미리보기 호출 (CharacterCreation 연동)
	var controller = _find_character_creation_controller()
	if controller and controller.has_method("preview_category_selection"):
		controller.preview_category_selection(category)

	_emit_data_update()


func _update_category_selection() -> void:
	# Update button pressed states
	for category in category_buttons:
		var btn = category_buttons[category]
		btn.button_pressed = (category == selected_category)


func _update_category_info_display() -> void:
	if selected_category.is_empty():
		info_display["category_title"].text = "카테고리를 선택하세요"
		info_display["category_description"].text = "위 3개 중 하나를 선택하면 해당 영역의 모든 능력치가 향상됩니다"
		info_display["features_list"].text = ""
		return

	var info = category_info[selected_category]
	info_display["category_title"].text = info.title + " " + info.subtitle
	info_display["category_description"].text = info.description

	var features_text = "강화 영역: " + " • ".join(info.features)
	info_display["features_list"].text = features_text


func _update_ca_display() -> void:
	info_display["current_ca"].text = "현재 CA: " + str(current_ca)

	if selected_category.is_empty():
		projected_ca = current_ca
		info_display["projected_ca"].text = "예상 CA: " + str(projected_ca)
	else:
		var bonus = category_info[selected_category].ca_bonus
		projected_ca = current_ca + bonus
		info_display["projected_ca"].text = "예상 CA: " + str(projected_ca) + " (+" + str(bonus) + ")"
		info_display["projected_ca"].modulate = Color(0.2, 0.8, 0.2)  # Green


func _calculate_current_ca() -> int:
	# Simplified CA calculation for display purposes
	if not character_data.has("detailed_attributes"):
		return 80

	var attributes = character_data.detailed_attributes
	var technical_sum = 0
	var mental_sum = 0
	var physical_sum = 0
	var gk_sum = 0

	# Technical attributes (14)
	var technical_attrs = category_info["Technical"].attributes
	for attr in technical_attrs:
		if attributes.has(attr):
			technical_sum += attributes[attr]

	# Mental attributes (14)
	var mental_attrs = category_info["Mental"].attributes
	for attr in mental_attrs:
		if attributes.has(attr):
			mental_sum += attributes[attr]

	# Physical attributes (8) - multiplied by 2 in CA calculation
	var physical_attrs = category_info["Physical"].attributes
	for attr in physical_attrs:
		if attributes.has(attr):
			physical_sum += attributes[attr]

	# GK attributes (6)
	var gk_attrs = ["reflexes", "handling", "aerial_ability", "command_of_area", "communication", "kicking"]
	for attr in gk_attrs:
		if attributes.has(attr):
			gk_sum += attributes[attr]

	var total_units = technical_sum + mental_sum + (physical_sum * 2) + gk_sum
	var base_ca = (total_units - 1000) / 20.0 if total_units >= 1000 else total_units / 40.0

	# Apply position modifier (simplified to 1.0 for now)
	return int(base_ca)


func _find_character_creation_controller():
	# Safe controller finding method - traverse up the tree
	var current_node = self
	for i in range(10):
		current_node = current_node.get_parent()
		if current_node == null:
			break
		# Check if this is the CharacterCreation controller by node name
		if current_node.name == "CharacterCreation":
			return current_node
		# Alternative: check by script path
		if current_node.get_script() != null:
			var script_path = current_node.get_script().resource_path
			if "CharacterCreation" in script_path:
				return current_node
		# Alternative: check by methods
		if current_node.has_method("preview_category_selection"):
			return current_node
	return null


func _emit_data_update() -> void:
	if selected_category.is_empty():
		return

	var data = {"selected_category": selected_category, "category_bonus_applied": true}
	emit_signal("data_updated", data)
