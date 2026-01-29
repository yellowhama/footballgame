extends Control

# Special Ability Screen - Power Pro Style with OpenFootball Integration
class_name SpecialAbilityScreen

# UI References
@onready var back_button = $VBoxContainer/Header/BackButton
@onready var unlock_count_label = $VBoxContainer/Header/UnlockCount
@onready var category_tabs = $VBoxContainer/CategoryTabs

# Grids for each category
@onready var technical_grid = $"VBoxContainer/CategoryTabs/Technical/TechnicalGrid"
@onready var mental_grid = $"VBoxContainer/CategoryTabs/Mental/MentalGrid"
@onready var physical_grid = $"VBoxContainer/CategoryTabs/Physical/PhysicalGrid"

# System references
var special_ability_system = null
var football_simulator = null
var player_data = null

# Current player skills for progress calculation
var current_skills = {}

# 7-tier system mapping from OpenFootball
const TIER_BADGES = {
	"Bronze": "🟤", "Silver": "⚪", "Gold": "🟡", "Diamond": "💎", "Legend": "🌈", "Red": "🔴", "Poison": "🟣"
}

const TIER_COLORS = {
	"Bronze": Color(0.6, 0.4, 0.2),
	"Silver": Color(0.8, 0.8, 0.8),
	"Gold": Color(1.0, 0.8, 0.2),
	"Diamond": Color(0.4, 0.8, 1.0),
	"Legend": Color(1.0, 0.4, 1.0),
	"Red": Color(1.0, 0.2, 0.2),
	"Poison": Color(0.6, 0.2, 0.8)
}


func _ready():
	_initialize_systems()
	_setup_ui()
	_load_player_data()
	_update_display()


func _initialize_systems():
	"""Initialize connections to game systems"""
	# Connect to SpecialAbilitySystem autoload
	special_ability_system = get_node_or_null("/root/SpecialAbilitySystem")
	if not special_ability_system:
		print("[SpecialAbilityScreen] Warning: SpecialAbilitySystem not found")

	# Initialize OpenFootball FootballSimulator
	if ClassDB.class_exists("FootballSimulator"):
		football_simulator = ClassDB.instantiate("FootballSimulator")
		print("[SpecialAbilityScreen] FootballSimulator initialized")
	else:
		print("[SpecialAbilityScreen] FootballSimulator class not found")


func _setup_ui():
	"""Setup UI components and generate ability cards"""
	# Technical abilities are already in the scene file
	# Add Mental and Physical ability cards dynamically
	_generate_mental_ability_cards()
	_generate_physical_ability_cards()

	# Connect signals
	if special_ability_system:
		special_ability_system.ability_unlocked.connect(_on_ability_unlocked)


func _generate_mental_ability_cards():
	"""Generate cards for Mental abilities"""
	var mental_abilities = [
		{
			"id": "CaptainMaterial",
			"name": "주장감",
			"description": "리더십 +30%, 팀워크 보너스",
			"unlock_condition": {"leadership": 85, "teamwork": 80},
			"tier": "Gold"
		},
		{
			"id": "ClutchPlayer",
			"name": "클러치 플레이어",
			"description": "중요한 순간 집중력 +25%",
			"unlock_condition": {"concentration": 80, "composure": 80},
			"tier": "Diamond"
		},
		{
			"id": "TeamPlayer",
			"name": "팀 플레이어",
			"description": "팀워크 +30%, 관계 향상",
			"unlock_condition": {"teamwork": 85, "leadership": 70},
			"tier": "Silver"
		},
		{
			"id": "PressureHandler",
			"name": "압박 관리자",
			"description": "압박 상황 침착함 +25%",
			"unlock_condition": {"composure": 80, "decisions": 75},
			"tier": "Gold"
		}
	]

	for ability_data in mental_abilities:
		var card = _create_ability_card(ability_data)
		mental_grid.add_child(card)


func _generate_physical_ability_cards():
	"""Generate cards for Physical abilities"""
	var physical_abilities = [
		{
			"id": "SpeedDemon",
			"name": "스피드 악마",
			"description": "스피드 +25% 향상",
			"unlock_condition": {"pace": 85, "acceleration": 80},
			"tier": "Diamond"
		},
		{
			"id": "EnduranceKing",
			"name": "체력왕",
			"description": "스태미나 +30%, 빠른 회복",
			"unlock_condition": {"stamina": 85, "natural_fitness": 80},
			"tier": "Legend"
		},
		{
			"id": "PowerHouse",
			"name": "파워하우스",
			"description": "힘 +25%, 점프력 증가",
			"unlock_condition": {"strength": 85, "jumping": 75},
			"tier": "Gold"
		},
		{
			"id": "AgilityMaster",
			"name": "민첩성 마스터",
			"description": "민첩성과 균형 +25%",
			"unlock_condition": {"agility": 85, "balance": 80},
			"tier": "Silver"
		}
	]

	for ability_data in physical_abilities:
		var card = _create_ability_card(ability_data)
		physical_grid.add_child(card)


func _create_ability_card(ability_data: Dictionary) -> Panel:
	"""Create a single ability card UI element"""
	var panel = Panel.new()
	panel.custom_minimum_size = Vector2(0, 180)

	var vbox = VBoxContainer.new()
	vbox.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	vbox.add_theme_constant_override("margin_left", 10)
	vbox.add_theme_constant_override("margin_top", 10)
	vbox.add_theme_constant_override("margin_right", -10)
	vbox.add_theme_constant_override("margin_bottom", -10)
	panel.add_child(vbox)

	# Header with tier badge, name, and lock icon
	var header = HBoxContainer.new()
	vbox.add_child(header)

	var tier_badge = Label.new()
	tier_badge.text = TIER_BADGES.get(ability_data.get("tier", "Bronze"), "🟤")
	tier_badge.add_theme_font_size_override("font_size", 28)
	header.add_child(tier_badge)

	var name_label = Label.new()
	name_label.text = ability_data.name
	name_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	name_label.add_theme_font_size_override("font_size", 20)
	header.add_child(name_label)

	var lock_icon = Label.new()
	lock_icon.text = "🔒"
	lock_icon.add_theme_font_size_override("font_size", 20)
	lock_icon.name = "LockIcon"
	header.add_child(lock_icon)

	# Description
	var description = Label.new()
	description.text = ability_data.description
	description.add_theme_font_size_override("font_size", 16)
	description.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	vbox.add_child(description)

	# Unlock conditions
	var unlock_container = VBoxContainer.new()
	vbox.add_child(unlock_container)

	var unlock_label = Label.new()
	unlock_label.text = "해금 조건:"
	unlock_label.add_theme_font_size_override("font_size", 14)
	unlock_container.add_child(unlock_label)

	# Add progress bars for each condition
	for skill_name in ability_data.unlock_condition:
		var required_value = ability_data.unlock_condition[skill_name]
		var progress_container = _create_progress_bar(skill_name, required_value)
		unlock_container.add_child(progress_container)

	# Store ability data in panel metadata
	panel.set_meta("ability_id", ability_data.id)
	panel.set_meta("ability_data", ability_data)

	return panel


func _create_progress_bar(skill_name: String, required_value: int) -> HBoxContainer:
	"""Create a progress bar for unlock condition"""
	var container = HBoxContainer.new()

	var label = Label.new()
	label.text = skill_name.capitalize() + ":"
	label.custom_minimum_size = Vector2(100, 0)
	label.add_theme_font_size_override("font_size", 14)
	container.add_child(label)

	var progress_bar = ProgressBar.new()
	progress_bar.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	progress_bar.max_value = float(required_value)
	progress_bar.value = 0.0
	progress_bar.show_percentage = false
	progress_bar.name = skill_name + "_progress"
	container.add_child(progress_bar)

	var value_label = Label.new()
	value_label.text = "0/" + str(required_value)
	value_label.add_theme_font_size_override("font_size", 14)
	value_label.name = skill_name + "_value"
	container.add_child(value_label)

	return container


func _load_player_data():
	"""Load current player data from OpenFootball"""
	if football_simulator:
		# Get player skills from OpenFootball
		var response = football_simulator.call("get_player_attributes_json", "{}")
		if response and response != "":
			var json = JSON.new()
			if json.parse(response) == OK:
				var data = json.data
				if data.has("attributes"):
					current_skills = data.attributes
					print("[SpecialAbilityScreen] Loaded %d player attributes" % current_skills.size())

	# Fallback to test data if no real data available
	if current_skills.is_empty():
		print("[SpecialAbilityScreen] Using test skill data")
		current_skills = _get_test_skills()


func _get_test_skills() -> Dictionary:
	"""Get test skill values for development"""
	return {
		# Technical
		"dribbling": 75,
		"technique": 68,
		"passing": 82,
		"vision": 70,
		"finishing": 77,
		"long_shots": 65,
		"free_kicks": 72,
		"corners": 70,
		# Mental
		"leadership": 60,
		"teamwork": 75,
		"concentration": 72,
		"composure": 68,
		"decisions": 70,
		"determination": 75,
		# Physical
		"pace": 80,
		"acceleration": 78,
		"stamina": 70,
		"natural_fitness": 72,
		"strength": 65,
		"jumping": 60,
		"agility": 75,
		"balance": 72
	}


func _update_display():
	"""Update the entire display with current data"""
	if not special_ability_system:
		return

	# Update unlock count
	var unlocked = special_ability_system.get_unlocked_abilities()
	unlock_count_label.text = "해금: %d/12" % unlocked.size()

	# Update each ability card
	_update_ability_cards(technical_grid, "Technical")
	_update_ability_cards(mental_grid, "Mental")
	_update_ability_cards(physical_grid, "Physical")

	# Check for new unlocks
	if special_ability_system:
		special_ability_system.check_ability_unlock(current_skills)


func _update_ability_cards(grid: GridContainer, category: String):
	"""Update all ability cards in a category"""
	for child in grid.get_children():
		if child is Panel and child.has_meta("ability_id"):
			var ability_id = child.get_meta("ability_id")
			var ability_data = child.get_meta("ability_data")

			# Check if unlocked
			var is_unlocked = false
			if special_ability_system:
				is_unlocked = special_ability_system.is_ability_unlocked(ability_id)

			# Update lock icon
			var lock_icon = child.find_child("LockIcon", true, false)
			if lock_icon:
				lock_icon.visible = not is_unlocked

			# Update progress bars
			_update_ability_progress(child, ability_data, is_unlocked)

			# Update panel appearance
			if is_unlocked:
				child.modulate = Color.WHITE
			else:
				child.modulate = Color(0.7, 0.7, 0.7, 0.9)


func _update_ability_progress(card: Panel, ability_data: Dictionary, is_unlocked: bool):
	"""Update progress bars for an ability card"""
	if is_unlocked:
		return  # No need to show progress for unlocked abilities

	var unlock_conditions = ability_data.get("unlock_condition", {})

	for skill_name in unlock_conditions:
		var required_value = unlock_conditions[skill_name]
		var current_value = current_skills.get(skill_name, 0)

		# Find and update progress bar
		var progress_bar = card.find_child(skill_name + "_progress", true, false)
		if progress_bar and progress_bar is ProgressBar:
			progress_bar.value = float(current_value)

			# Color based on progress
			if current_value >= required_value:
				progress_bar.modulate = Color(0.2, 1.0, 0.2)
			elif current_value >= required_value * 0.8:
				progress_bar.modulate = Color(1.0, 1.0, 0.2)
			else:
				progress_bar.modulate = Color.WHITE

		# Update value label
		var value_label = card.find_child(skill_name + "_value", true, false)
		if value_label and value_label is Label:
			value_label.text = "%d/%d" % [current_value, required_value]
			if current_value >= required_value:
				value_label.modulate = Color(0.2, 1.0, 0.2)
			else:
				value_label.modulate = Color.WHITE


func _on_ability_unlocked(ability_id: String):
	"""Handle ability unlock notification"""
	print("[SpecialAbilityScreen] Ability unlocked: %s" % ability_id)
	_update_display()

	# Show unlock effect
	if has_node("/root/UIService"):
		var ui_service = get_node("/root/UIService")
		ui_service.show_success_toast("🎉 특수능력 해금: " + ability_id)


func _on_back_button_pressed():
	"""Return to previous screen"""
	print("[SpecialAbilityScreen] Back button pressed")
	# Return to status screen or main menu
	get_tree().change_scene_to_file("res://scenes/StatusScreenImproved.tscn")


func _on_combine_button_pressed():
	"""Open combination UI"""
	print("[SpecialAbilityScreen] Combine button pressed")
	# TODO: Implement combination UI
	if has_node("/root/UIService"):
		var ui_service = get_node("/root/UIService")
		ui_service.show_toast("조합 시스템 준비중...", 2.0)


func _on_detail_button_pressed():
	"""Show detailed ability information"""
	print("[SpecialAbilityScreen] Detail button pressed")
	# TODO: Show detailed popup
	if has_node("/root/UIService"):
		var ui_service = get_node("/root/UIService")
		ui_service.open_modal(
			"특수능력 상세", "Power Pro 스타일 특수능력 시스템\n\n7단계 티어:\n🟤 Bronze → ⚪ Silver → 🟡 Gold → 💎 Diamond → 🌈 Legend", ["확인"]
		)


func _on_test_button_pressed():
	"""Test unlock abilities (development only)"""
	print("[SpecialAbilityScreen] Test unlock pressed")

	# Boost some skills to trigger unlocks
	current_skills["dribbling"] = 85
	current_skills["technique"] = 75
	current_skills["passing"] = 90
	current_skills["vision"] = 80

	# Check for unlocks with boosted skills
	if special_ability_system:
		var new_unlocks = special_ability_system.check_ability_unlock(current_skills)
		if new_unlocks.size() > 0:
			print("[SpecialAbilityScreen] Test unlocked: ", new_unlocks)

	_update_display()


# Integration with OpenFootball API
func get_special_abilities_from_openfootball() -> Array:
	"""Get special abilities from OpenFootball backend"""
	if not football_simulator:
		return []

	var request = {"player_id": 0}  # Current player
	var response = football_simulator.call("get_special_abilities_json", JSON.stringify(request))

	if response and response != "":
		var json = JSON.new()
		if json.parse(response) == OK:
			return json.data.get("abilities", [])

	return []


func apply_tier_to_ability(ability_id: String, tier: String):
	"""Apply OpenFootball tier to an ability"""
	if not football_simulator:
		return

	var request = {"ability_type": ability_id, "tier": tier}

	var response = football_simulator.call("set_special_ability_tier_json", JSON.stringify(request))
	print("[SpecialAbilityScreen] Set tier response: ", response)
