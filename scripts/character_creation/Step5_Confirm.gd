extends Control

signal data_updated(data: Dictionary)
signal edit_requested(step: int)

var character_data: Dictionary = {}
var hair_styles = ["짧은 머리", "긴 머리", "곱슬머리", "대머리", "포마드", "투블록", "모히칸", "웨이브"]

@onready
var head_label: Label = $ScrollContainer/VBoxContainer/PreviewSection/CenterContainer/CharacterPreview/VBoxContainer/Head
@onready
var body_label: Label = $ScrollContainer/VBoxContainer/PreviewSection/CenterContainer/CharacterPreview/VBoxContainer/Body

# Basic Info
@onready
var name_label: Label = $ScrollContainer/VBoxContainer/BasicInfoSection/MarginContainer/VBoxContainer/NameRow/ValueLabel
@onready
var position_label: Label = $ScrollContainer/VBoxContainer/BasicInfoSection/MarginContainer/VBoxContainer/PositionRow/ValueLabel
@onready
var number_label: Label = $ScrollContainer/VBoxContainer/BasicInfoSection/MarginContainer/VBoxContainer/NumberRow/ValueLabel
@onready
var basic_edit_button: Button = $ScrollContainer/VBoxContainer/BasicInfoSection/MarginContainer/VBoxContainer/SectionTitle/EditButton

# Appearance
@onready
var hair_label: Label = $ScrollContainer/VBoxContainer/AppearanceSection/MarginContainer/VBoxContainer/HairRow/ValueLabel
@onready
var body_type_label: Label = $ScrollContainer/VBoxContainer/AppearanceSection/MarginContainer/VBoxContainer/BodyRow/ValueLabel
@onready
var appearance_edit_button: Button = $ScrollContainer/VBoxContainer/AppearanceSection/MarginContainer/VBoxContainer/SectionTitle/EditButton

# Stats
@onready
var pace_label: Label = $ScrollContainer/VBoxContainer/StatsSection/MarginContainer/VBoxContainer/GridContainer/PaceLabel
@onready
var shooting_label: Label = $ScrollContainer/VBoxContainer/StatsSection/MarginContainer/VBoxContainer/GridContainer/ShootingLabel
@onready
var passing_label: Label = $ScrollContainer/VBoxContainer/StatsSection/MarginContainer/VBoxContainer/GridContainer/PassingLabel
@onready
var dribbling_label: Label = $ScrollContainer/VBoxContainer/StatsSection/MarginContainer/VBoxContainer/GridContainer/DribblingLabel
@onready
var defending_label: Label = $ScrollContainer/VBoxContainer/StatsSection/MarginContainer/VBoxContainer/GridContainer/DefendingLabel
@onready
var physical_label: Label = $ScrollContainer/VBoxContainer/StatsSection/MarginContainer/VBoxContainer/GridContainer/PhysicalLabel
@onready
var stats_edit_button: Button = $ScrollContainer/VBoxContainer/StatsSection/MarginContainer/VBoxContainer/SectionTitle/EditButton
@onready var confirm_button: Button = $ScrollContainer/VBoxContainer/ConfirmButton


func _ready() -> void:
	print("[Step5_Confirm] Ready")
	_connect_signals()
	_update_display()


func _connect_signals() -> void:
	basic_edit_button.pressed.connect(_on_edit_basic_info)
	appearance_edit_button.pressed.connect(_on_edit_appearance)
	stats_edit_button.pressed.connect(_on_edit_stats)

	# 생성 완료 버튼 - 부모 컨트롤러의 _complete_creation() 호출
	if confirm_button:
		confirm_button.pressed.connect(_on_confirm_button_pressed)
		print("[Step5_Confirm] ✅ Confirm button connected")


func set_character_data(data: Dictionary) -> void:
	character_data = data
	if is_node_ready():
		_update_display()


func update_display(data: Dictionary) -> void:
	character_data = data
	_update_display()


func _update_display() -> void:
	# Update Basic Info
	if character_data.has("basic_info"):
		var info = character_data.basic_info
		if info.has("name"):
			name_label.text = info.name if info.name != "" else "이름 없음"
		if info.has("position"):
			position_label.text = info.position
		if info.has("number"):
			number_label.text = str(info.number)

	# Update Appearance
	if character_data.has("appearance"):
		var appearance = character_data.appearance
		if appearance.has("hair_style") and appearance.hair_style < hair_styles.size():
			hair_label.text = hair_styles[appearance.hair_style]
		if appearance.has("body_type"):
			match appearance.body_type:
				0:
					body_type_label.text = "마른 체형"
				1:
					body_type_label.text = "보통 체형"
				2:
					body_type_label.text = "건장한 체형"

		# Update preview
		if appearance.has("face_preset"):
			var faces = ["😀", "😄", "😎", "🤩", "😐", "🤔"]
			if appearance.face_preset < faces.size():
				head_label.text = faces[appearance.face_preset]

		if appearance.has("body_type"):
			match appearance.body_type:
				0:
					body_label.text = "🎽"
				1:
					body_label.text = "👕"
				2:
					body_label.text = "💪"

	# Update Stats
	if character_data.has("stats"):
		var stats = character_data.stats
		if stats.has("pace"):
			pace_label.text = "속력: " + str(stats.pace)
			_update_stat_color(pace_label, stats.pace)
		if stats.has("shooting"):
			shooting_label.text = "슈팅: " + str(stats.shooting)
			_update_stat_color(shooting_label, stats.shooting)
		if stats.has("passing"):
			passing_label.text = "패스: " + str(stats.passing)
			_update_stat_color(passing_label, stats.passing)
		if stats.has("dribbling"):
			dribbling_label.text = "드리블: " + str(stats.dribbling)
			_update_stat_color(dribbling_label, stats.dribbling)
		if stats.has("defending"):
			defending_label.text = "수비: " + str(stats.defending)
			_update_stat_color(defending_label, stats.defending)
		if stats.has("physical"):
			physical_label.text = "피지컬: " + str(stats.physical)
			_update_stat_color(physical_label, stats.physical)


func _update_stat_color(label: Label, value: int) -> void:
	if value >= 65:
		label.modulate = Color(0.2, 0.8, 0.2)  # Green
	elif value >= 55:
		label.modulate = Color(1, 0.84, 0)  # Yellow
	elif value >= 45:
		label.modulate = Color(1, 1, 1)  # White
	else:
		label.modulate = Color(0.8, 0.3, 0.3)  # Red


func _on_edit_basic_info() -> void:
	print("[Step5_Confirm] Edit basic info requested")
	# Navigate back to Step 1
	if get_parent() and get_parent().has_method("_load_step"):
		get_parent()._load_step(0)
		get_parent()._update_step_indicator()


func _on_edit_appearance() -> void:
	print("[Step5_Confirm] Edit appearance requested")
	# Navigate back to Step 2
	if get_parent() and get_parent().has_method("_load_step"):
		get_parent()._load_step(1)
		get_parent()._update_step_indicator()


func _on_edit_stats() -> void:
	print("[Step5_Confirm] Edit stats requested")
	# Navigate back to Step 3
	if get_parent() and get_parent().has_method("_load_step"):
		get_parent()._load_step(2)
		get_parent()._update_step_indicator()


func _emit_data_update() -> void:
	# Step 5 doesn't update data, just confirms
	pass


func _on_confirm_button_pressed() -> void:
	print("[Step5_Confirm] ========================================")
	print("[Step5_Confirm] Confirm button pressed!")
	print("[Step5_Confirm] Getting parent controller...")

	# 부모 컨트롤러 찾기
	var parent = get_parent()
	while parent != null:
		if parent.has_method("_complete_creation"):
			print("[Step5_Confirm] Found controller, calling _complete_creation()")
			parent._complete_creation()
			return
		parent = parent.get_parent()

	print("[Step5_Confirm] ERROR: Could not find parent with _complete_creation()")
	print("[Step5_Confirm] ========================================")
