extends Control

signal data_updated(data: Dictionary)

var character_data: Dictionary = {}
var selected_category: String = ""
var selected_specialties: Array = []
var current_ca: int = 80
var projected_ca: int = 80

@onready var specialty_buttons: Dictionary = {}
@onready var info_display: Dictionary = {}

# Attribute display names (Korean)
var attribute_names: Dictionary = {
	# Technical
	"dribbling": "드리블",
	"passing": "패스",
	"shooting": "슛",
	"crossing": "크로스",
	"first_touch": "퍼스트터치",
	"ball_control": "볼 컨트롤",
	"technique": "테크닉",
	"heading": "헤딩",
	"finishing": "마무리",
	"long_shots": "중거리슛",
	"free_kicks": "프리킥",
	"penalties": "페널티킥",
	"corners": "코너킥",
	"throw_ins": "스로인",
	# Mental
	"decisions": "판단력",
	"concentration": "집중력",
	"leadership": "리더십",
	"vision": "시야",
	"teamwork": "팀워크",
	"work_rate": "활동량",
	"positioning": "위치선정",
	"anticipation": "예측력",
	"composure": "침착성",
	"bravery": "용기",
	"determination": "의지력",
	"flair": "창의성",
	"off_the_ball": "오프더볼",
	"aggression": "적극성",
	# Physical
	"speed": "스피드",
	"stamina": "스태미너",
	"strength": "힘",
	"agility": "민첩성",
	"balance": "밸런스",
	"jumping": "점프력",
	"natural_fitness": "체력",
	"acceleration": "가속력"
}

# Category information from Step3
var category_info: Dictionary = {
	"Technical":
	{
		"title": "⚙️ Technical 특장점",
		"subtitle": "기술적 능력에서 가장 뛰어난 3가지를 선택하세요",
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
		]
	},
	"Mental":
	{
		"title": "🧠 Mental 특장점",
		"subtitle": "정신적 능력에서 가장 뛰어난 3가지를 선택하세요",
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
		]
	},
	"Physical":
	{
		"title": "💪 Physical 특장점",
		"subtitle": "신체적 능력에서 가장 뛰어난 3가지를 선택하세요",
		"attributes":
		["speed", "stamina", "strength", "agility", "balance", "jumping", "natural_fitness", "acceleration"]
	}
}


func _ready() -> void:
	print("[Step4_SpecialtyChoice] Ready")
	_setup_controls()
	_connect_signals()


func _setup_controls() -> void:
	# Get selected category from character data
	if character_data.has("selected_category"):
		selected_category = character_data.selected_category

	if selected_category.is_empty():
		print("[Step4_SpecialtyChoice] Warning: No category selected")
		return

	# Setup header info with null checks
	var info = category_info[selected_category]
	var title_label = $ScrollContainer/VBoxContainer/Header/TitleLabel
	var subtitle_label = $ScrollContainer/VBoxContainer/Header/SubtitleLabel

	if title_label:
		title_label.text = info.title
	else:
		print("[Step4_SpecialtyChoice] Error: TitleLabel not found")

	if subtitle_label:
		subtitle_label.text = info.subtitle
	else:
		print("[Step4_SpecialtyChoice] Error: SubtitleLabel not found")

	# Setup specialty buttons dynamically
	var buttons_container = $ScrollContainer/VBoxContainer/SpecialtyButtons
	var attributes = info.attributes

	# Clear existing buttons first to prevent duplication
	for child in buttons_container.get_children():
		child.queue_free()
	specialty_buttons.clear()

	# Preserve existing selections or initialize empty
	if character_data.has("selected_specialties") and character_data.selected_specialties is Array:
		selected_specialties = character_data.selected_specialties.duplicate()
		print("[Step4_SpecialtyChoice] Loaded existing specialties: ", selected_specialties)
	else:
		selected_specialties.clear()
		print("[Step4_SpecialtyChoice] No existing specialties, starting fresh")

	# Create GridContainer for 3x5 layout
	var grid = GridContainer.new()
	grid.columns = 3
	grid.add_theme_constant_override("h_separation", 10)
	grid.add_theme_constant_override("v_separation", 10)
	buttons_container.add_child(grid)

	# Create buttons for each attribute in selected category
	for i in range(attributes.size()):
		var attr = attributes[i]
		var btn = Button.new()
		btn.name = attr + "Button"
		btn.text = attribute_names.get(attr, attr)  # Just the Korean name
		btn.custom_minimum_size = Vector2(200, 100)  # Wider and taller buttons
		btn.toggle_mode = true  # Make it toggleable like checkbox

		# Set button style
		btn.add_theme_font_size_override("font_size", 24)

		# Set initial state based on selected_specialties
		if attr in selected_specialties:
			btn.button_pressed = true

		grid.add_child(btn)
		specialty_buttons[attr] = btn

		# Connect signal immediately after creating button
		btn.toggled.connect(_on_specialty_toggled.bind(attr))

	# Setup info display elements with null checks
	var current_ca_label = $ScrollContainer/VBoxContainer/Header/CADisplay/CurrentCA
	var projected_ca_label = $ScrollContainer/VBoxContainer/Header/CADisplay/ProjectedCA
	var count_label = $ScrollContainer/VBoxContainer/Header/SelectionInfo/CountLabel
	var hint_label = $ScrollContainer/VBoxContainer/Header/SelectionInfo/HintLabel

	if current_ca_label:
		info_display["current_ca"] = current_ca_label
	else:
		print("[Step4_SpecialtyChoice] Error: CurrentCA label not found")

	if projected_ca_label:
		info_display["projected_ca"] = projected_ca_label
	else:
		print("[Step4_SpecialtyChoice] Error: ProjectedCA label not found")

	if count_label:
		info_display["selection_count"] = count_label
	else:
		print("[Step4_SpecialtyChoice] Error: CountLabel not found")

	if hint_label:
		info_display["selection_hint"] = hint_label
	else:
		print("[Step4_SpecialtyChoice] Error: HintLabel not found")

	current_ca = _calculate_current_ca()
	_update_ca_display()
	_update_selection_info()


func _connect_signals() -> void:
	# Signals are now connected directly when buttons are created in _setup_controls()
	pass


func set_character_data(data: Dictionary) -> void:
	character_data = data
	if is_node_ready():
		_setup_controls()


func update_display(data: Dictionary) -> void:
	character_data = data
	_setup_controls()


func _on_specialty_toggled(pressed: bool, attr: String) -> void:
	print("[Step4_SpecialtyChoice] Toggled: ", attr, " = ", pressed)

	if pressed:
		# Add to selection if not at limit and not already in list
		if attr not in selected_specialties:
			if selected_specialties.size() < 3:
				selected_specialties.append(attr)
				print("[Step4_SpecialtyChoice] Added ", attr, ". Selected: ", selected_specialties)
			else:
				# Limit reached, uncheck the button
				specialty_buttons[attr].button_pressed = false
				print("[Step4_SpecialtyChoice] Limit reached, cannot add ", attr)
				return
	else:
		# Remove from selection
		if attr in selected_specialties:
			selected_specialties.erase(attr)
			print("[Step4_SpecialtyChoice] Removed ", attr, ". Selected: ", selected_specialties)

	_update_selection_info()
	_update_ca_display()

	# 실시간 CA 미리보기 호출 (CharacterCreation 연동)
	var controller = _find_character_creation_controller()
	if controller and controller.has_method("preview_specialty_selection"):
		controller.preview_specialty_selection(selected_specialties)

	_emit_data_update()


func _update_selection_info() -> void:
	var count = selected_specialties.size()

	# Safe access to selection_count label
	if info_display.has("selection_count") and info_display["selection_count"]:
		info_display["selection_count"].text = "선택됨: " + str(count) + "/3"

	# Safe access to selection_hint label
	if info_display.has("selection_hint") and info_display["selection_hint"]:
		if count == 0:
			info_display["selection_hint"].text = "3개의 특장점을 선택하세요"
			info_display["selection_hint"].modulate = Color.WHITE
		elif count < 3:
			info_display["selection_hint"].text = str(3 - count) + "개 더 선택하세요"
			info_display["selection_hint"].modulate = Color(1.0, 0.84, 0.0)  # Yellow
		else:
			info_display["selection_hint"].text = "✓ 선택 완료!"
			info_display["selection_hint"].modulate = Color(0.2, 0.8, 0.2)  # Green


func _update_ca_display() -> void:
	# Safe access to current_ca label
	if info_display.has("current_ca") and info_display["current_ca"]:
		info_display["current_ca"].text = "현재 CA: " + str(current_ca)

	var specialty_bonus = selected_specialties.size() * 1  # Each specialty adds ~1 CA
	if selected_category == "Physical":
		specialty_bonus = selected_specialties.size() * 2  # Physical has *2 multiplier

	projected_ca = current_ca + specialty_bonus

	# Safe access to projected_ca label
	if info_display.has("projected_ca") and info_display["projected_ca"]:
		if specialty_bonus > 0:
			info_display["projected_ca"].text = "예상 CA: " + str(projected_ca) + " (+" + str(specialty_bonus) + ")"
			info_display["projected_ca"].modulate = Color(0.2, 0.8, 0.2)  # Green
		else:
			info_display["projected_ca"].text = "예상 CA: " + str(projected_ca)
			info_display["projected_ca"].modulate = Color.WHITE


func _get_attribute_display_text(attr: String) -> String:
	# Just return the Korean name for simplicity
	return attribute_names.get(attr, attr)


func _calculate_current_ca() -> int:
	# Use simplified calculation - should match Step3's calculation
	if not character_data.has("detailed_attributes"):
		return 80

	# This should include category bonus if already applied
	var attributes = character_data.detailed_attributes
	var technical_sum = 0
	var mental_sum = 0
	var physical_sum = 0
	var gk_sum = 0

	# Calculate with current values (including category bonus)
	var technical_attrs = category_info["Technical"].attributes
	for attr in technical_attrs:
		if attributes.has(attr):
			technical_sum += attributes[attr]

	var mental_attrs = category_info["Mental"].attributes
	for attr in mental_attrs:
		if attributes.has(attr):
			mental_sum += attributes[attr]

	var physical_attrs = category_info["Physical"].attributes
	for attr in physical_attrs:
		if attributes.has(attr):
			physical_sum += attributes[attr]

	var gk_attrs = ["reflexes", "handling", "aerial_ability", "command_of_area", "communication", "kicking"]
	for attr in gk_attrs:
		if attributes.has(attr):
			gk_sum += attributes[attr]

	var total_units = technical_sum + mental_sum + (physical_sum * 2) + gk_sum
	var base_ca = (total_units - 1000) // 20 if total_units >= 1000 else total_units // 40

	return int(base_ca)


func _emit_data_update() -> void:
	var data = {
		"selected_specialties": selected_specialties, "specialty_selection_complete": selected_specialties.size() == 3
	}
	print("[Step4_SpecialtyChoice] Emitting data_updated with: ", data)
	emit_signal("data_updated", data)


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
		if current_node.has_method("preview_specialty_selection"):
			return current_node
	return null
