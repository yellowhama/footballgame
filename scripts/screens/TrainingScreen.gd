extends Control
## TrainingScreen - 훈련 화면 (TrainingManager.gd 연동)
## 디자인: 04_ui_design_system.md 기반
##
## 작성일: 2025-11-26

signal back_requested
signal training_executed(result: Dictionary)

# ============================================
# UI 노드 참조
# ============================================

@onready var back_button: Button = $Header/BackButton
@onready var title_label: Label = $Header/TitleLabel
@onready var condition_bar: ProgressBar = $Header/ConditionBar
@onready var condition_label: Label = $Header/ConditionLabel
@onready var training_limit_label: Label = $Header/TrainingLimitLabel

@onready var category_tabs: TabContainer = $Content/CategoryTabs
@onready var general_grid: GridContainer = $Content/CategoryTabs/General/ScrollContainer/Grid
@onready var personal_grid: GridContainer = $Content/CategoryTabs/Personal/ScrollContainer/Grid
@onready var special_grid: GridContainer = $Content/CategoryTabs/Special/ScrollContainer/Grid

@onready var intensity_selector: HBoxContainer = $Footer/IntensitySelector
@onready var light_button: Button = $Footer/IntensitySelector/LightButton
@onready var normal_button: Button = $Footer/IntensitySelector/NormalButton
@onready var intense_button: Button = $Footer/IntensitySelector/IntenseButton

@onready var rest_button: Button = $Footer/ActionButtons/RestButton
@onready var go_out_button: Button = $Footer/ActionButtons/GoOutButton

# ============================================
# 상태 변수
# ============================================

var _selected_training_id: String = ""
var _selected_intensity: String = "normal"
var _training_cards: Dictionary = {}  # training_id -> TrainingCard node
var _result_popup: Control = null

const TrainingCardScene = preload("res://scenes/components/TrainingCard.tscn")
const TrainingResultPopupScene = preload("res://scenes/ui/TrainingResultPopup.tscn")
const MainNavBarScene = preload("res://scenes/components/MainNavBar.tscn")

# Design System Colors
const COLOR_BG_PRIMARY = Color("#0D1117")
const COLOR_BG_SECONDARY = Color("#161B22")
const COLOR_ACCENT_PRIMARY = Color("#238636")
const COLOR_ACCENT_SECONDARY = Color("#1F6FEB")
const COLOR_ACCENT_WARNING = Color("#D29922")
const COLOR_ACCENT_DANGER = Color("#DA3633")
const COLOR_TEXT_PRIMARY = Color("#E6EDF3")
const COLOR_TEXT_SECONDARY = Color("#8B949E")

# Training type icons
const TRAINING_ICONS = {
	"technical": "⚽", "physical": "💪", "tactical": "🧠", "defensive": "🛡️", "mental": "🎯", "special": "⭐"
}

# ============================================
# 초기화
# ============================================


func _ready() -> void:
	_connect_signals()
	_setup_ui()
	_load_trainings()
	_update_condition_display()
	_update_training_limit_display()
	_add_navigation_bar()
	print("[TrainingScreen] Initialized with TrainingManager integration")


func _add_navigation_bar() -> void:
	if MainNavBarScene:
		var navbar = MainNavBarScene.instantiate()
		add_child(navbar)
		navbar.set_active_tab("training")


func _connect_signals() -> void:
	# Header buttons
	if back_button:
		back_button.pressed.connect(_on_back_pressed)

	# Intensity buttons
	if light_button:
		light_button.pressed.connect(_on_intensity_selected.bind("light"))
	if normal_button:
		normal_button.pressed.connect(_on_intensity_selected.bind("normal"))
	if intense_button:
		intense_button.pressed.connect(_on_intensity_selected.bind("intense"))

	# Action buttons
	if rest_button:
		rest_button.pressed.connect(_on_rest_pressed)
	if go_out_button:
		go_out_button.pressed.connect(_on_go_out_pressed)

	# TrainingManager signals
	if TrainingManager:
		TrainingManager.training_completed.connect(_on_training_completed)
		TrainingManager.training_failed.connect(_on_training_failed)
		TrainingManager.rest_activity_completed.connect(_on_rest_completed)
		TrainingManager.go_out_activity_completed.connect(_on_go_out_completed)


func _setup_ui() -> void:
	# Apply dark theme colors
	if has_node("Background"):
		$Background.color = COLOR_BG_PRIMARY

	# Set initial intensity selection
	_update_intensity_buttons()


# ============================================
# 훈련 목록 로드
# ============================================


func _load_trainings() -> void:
	if not TrainingManager:
		push_error("[TrainingScreen] TrainingManager not found!")
		return

	var all_trainings = TrainingManager.get_available_trainings()

	# Categorize trainings
	var general_trainings = []
	var personal_trainings = []
	var special_trainings = []

	for training in all_trainings:
		var training_type = training.get("type", "")
		var training_id = training.get("id", "")

		if training_id.begins_with("special_"):
			special_trainings.append(training)
		elif training_type in ["technical", "physical", "tactical", "defensive"]:
			# Basic 6 trainings
			if training_id in ["shooting", "passing", "dribbling", "physical", "tactical", "defending"]:
				general_trainings.append(training)
			else:
				personal_trainings.append(training)
		else:
			personal_trainings.append(training)

	# Populate grids
	_populate_grid(general_grid, general_trainings, false)
	_populate_grid(personal_grid, personal_trainings, true)
	_populate_grid(special_grid, special_trainings, true)

	print(
		(
			"[TrainingScreen] Loaded %d general, %d personal, %d special trainings"
			% [general_trainings.size(), personal_trainings.size(), special_trainings.size()]
		)
	)


func _populate_grid(grid: GridContainer, trainings: Array, is_personal: bool) -> void:
	if not grid:
		return

	# Clear existing cards
	for child in grid.get_children():
		child.queue_free()

	# Create cards
	for training in trainings:
		var card = TrainingCardScene.instantiate()
		grid.add_child(card)

		var training_id = training.get("id", "")
		var training_type = training.get("type", "technical")
		var icon = TRAINING_ICONS.get(training_type, "📋")

		# Prepare card data
		var card_data = {
			"id": training_id,
			"name": training.get("name", training_id),
			"icon": icon,
			"attributes": training.get("attributes", {}),
			"condition_cost": training.get("condition_cost", 0),
			"fatigue_cost": training.get("condition_cost", 0),  # For TrainingCard compatibility
			"description": training.get("description", ""),
			"is_personal": is_personal
		}

		card.setup(card_data)
		card.selected.connect(_on_training_card_selected)
		_training_cards[training_id] = card

		# Check if training is available
		var can_train = TrainingManager.can_execute_training(training_id, "personal" if is_personal else "team")
		if not can_train.get("can_train", true):
			card.modulate = Color(0.5, 0.5, 0.5, 0.7)


# ============================================
# 상태 표시 업데이트
# ============================================


func _update_condition_display() -> void:
	if not ConditionSystem:
		return

	var condition = ConditionSystem.get_condition_percentage()

	if condition_bar:
		condition_bar.value = condition
		# Color based on condition level
		if condition >= 70:
			condition_bar.modulate = COLOR_ACCENT_PRIMARY
		elif condition >= 40:
			condition_bar.modulate = COLOR_ACCENT_WARNING
		else:
			condition_bar.modulate = COLOR_ACCENT_DANGER

	if condition_label:
		condition_label.text = "컨디션: %.0f%%" % condition


func _update_training_limit_display() -> void:
	if not TrainingManager:
		return

	var stats = TrainingManager.get_training_stats()
	var completed = stats.get("personal_trainings_completed", 0)
	var max_limit = TrainingManager.MAX_PERSONAL_TRAININGS_PER_WEEK

	if training_limit_label:
		training_limit_label.text = "개인 훈련: %d/%d" % [completed, max_limit]
		if completed >= max_limit:
			training_limit_label.add_theme_color_override("font_color", COLOR_ACCENT_DANGER)
		else:
			training_limit_label.add_theme_color_override("font_color", COLOR_TEXT_SECONDARY)


func _update_intensity_buttons() -> void:
	# Reset all buttons
	for btn in [light_button, normal_button, intense_button]:
		if btn:
			btn.add_theme_color_override("font_color", COLOR_TEXT_SECONDARY)

	# Highlight selected
	var selected_btn: Button = null
	match _selected_intensity:
		"light":
			selected_btn = light_button
		"normal":
			selected_btn = normal_button
		"intense":
			selected_btn = intense_button

	if selected_btn:
		selected_btn.add_theme_color_override("font_color", COLOR_ACCENT_SECONDARY)


# ============================================
# 이벤트 핸들러
# ============================================


func _on_back_pressed() -> void:
	back_requested.emit()
	get_tree().change_scene_to_file("res://scenes/HomeImproved.tscn")


func _on_intensity_selected(intensity: String) -> void:
	_selected_intensity = intensity
	TrainingManager.set_intensity(intensity)
	_update_intensity_buttons()
	print("[TrainingScreen] Intensity set to: %s" % intensity)


func _on_training_card_selected(card_data: Dictionary) -> void:
	var training_id = card_data.get("id", "")
	var is_personal = card_data.get("is_personal", true)

	if training_id.is_empty():
		return

	print("[TrainingScreen] Training selected: %s (personal: %s)" % [training_id, is_personal])

	# Check if can execute
	var check = TrainingManager.can_execute_training(training_id, "personal" if is_personal else "team")
	if not check.get("can_train", false):
		_show_error(check.get("reason", "훈련을 실행할 수 없습니다"))
		return

	# Execute training
	var result = TrainingManager.execute_training(training_id, is_personal)

	if result.get("success", false):
		_on_training_success(result)
	else:
		_show_error(result.get("message", "훈련 실패"))


func _on_training_success(result: Dictionary) -> void:
	print("[TrainingScreen] Training success: %s" % result)
	training_executed.emit(result)

	# Update UI
	_update_condition_display()
	_update_training_limit_display()

	# Show result popup or transition to result screen
	_show_training_result(result)


func _on_training_completed(event: Dictionary) -> void:
	print("[TrainingScreen] Training completed event: %s" % event)
	_update_condition_display()
	_update_training_limit_display()


func _on_training_failed(event: Dictionary) -> void:
	var error_msg = event.get("message", "훈련 실패")
	_show_error(error_msg)


func _on_rest_pressed() -> void:
	print("[TrainingScreen] Rest button pressed")
	var result = await TrainingManager.perform_rest_activity()
	if result.get("success", false):
		_update_condition_display()
		_show_message("휴식 완료! 컨디션이 회복되었습니다.")
	else:
		_show_error(result.get("message", "휴식 실패"))


func _on_go_out_pressed() -> void:
	print("[TrainingScreen] Go out button pressed")
	var result = TrainingManager.perform_go_out_activity()
	if result.get("success", false):
		_update_condition_display()
		_show_message("외출 완료! 기분이 좋아졌습니다.")
	else:
		_show_error(result.get("message", "외출 실패"))


func _on_rest_completed(result: Dictionary) -> void:
	_update_condition_display()


func _on_go_out_completed(result: Dictionary) -> void:
	_update_condition_display()


# ============================================
# UI 피드백
# ============================================


func _show_training_result(result: Dictionary) -> void:
	# 팝업 인스턴스 생성 (없으면)
	if not _result_popup:
		_result_popup = TrainingResultPopupScene.instantiate()
		add_child(_result_popup)
		_result_popup.closed.connect(_on_result_popup_closed)

	# 결과 표시
	_result_popup.show_result(result)


func _on_result_popup_closed() -> void:
	# 팝업 닫힌 후 추가 작업 (필요시)
	print("[TrainingScreen] Result popup closed")


func _show_message(text: String) -> void:
	# Simple notification - can be replaced with proper popup
	print("[TrainingScreen] Message: %s" % text)
	# TODO: Show toast/popup notification


func _show_error(text: String) -> void:
	push_warning("[TrainingScreen] Error: %s" % text)
	# TODO: Show error toast/popup


# ============================================
# 외부 API
# ============================================


func refresh() -> void:
	"""Refresh all training data and UI"""
	_load_trainings()
	_update_condition_display()
	_update_training_limit_display()
