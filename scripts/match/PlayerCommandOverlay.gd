extends Control
class_name PlayerCommandOverlay
##
## PlayerCommandOverlay
##
## Phase 4.6: Hero Time 액션 선택 오버레이
##
## Hero Time pause 발생 시 표시되어 유저가 액션을 선택할 수 있게 함.
## - 슛, 패스, 드리블 등 옵션 버튼 제공
## - 각 옵션의 성공률/리스크 표시
## - 선택 시 action_selected 시그널 발생
##

signal action_selected(action: Dictionary)

@onready var title_label: Label = $Panel/VBox/TitleLabel
@onready var context_label: Label = $Panel/VBox/ContextLabel
@onready var state_label: Label = $Panel/VBox/StateLabel
@onready var options_container: VBoxContainer = $Panel/VBox/OptionsContainer

## State to Korean display text mapping
const STATE_DISPLAY := {
	"WithBall": "⚽ 공 소유 중",
	"Attacking": "⚔️ 공격 중",
	"Defending": "🛡️ 수비 중",
}

var _decision_context: Dictionary = {}
var _session_controller: Node = null


func _ready() -> void:
	visible = false

	# Auto-hide on escape key
	set_process_unhandled_input(true)


func _unhandled_input(event: InputEvent) -> void:
	if visible and event.is_action_pressed("ui_cancel"):
		# Don't hide - user must select an action
		pass


## Show overlay with decision context from the engine
func show_decision(context: Dictionary, controller: Node = null) -> void:
	_decision_context = context
	_session_controller = controller

	# Update title
	var player_name: String = context.get("player_name", "내 선수")
	title_label.text = "⚡ HERO TIME - %s" % player_name

	# Update context info
	var time_sec: float = context.get("time_seconds", 0.0)
	var minute: int = int(time_sec / 60.0)
	context_label.text = "%d분 - 공을 잡았다! 다음 행동을 선택하세요." % minute

	# Update player state display (Phase 6 P2)
	var player_state: String = context.get("player_state", "")
	if player_state.is_empty():
		# Fallback: check for action field from adapter
		player_state = context.get("action", "")
	if player_state.is_empty():
		# Hero Time always triggers when player has the ball
		player_state = "WithBall"
	_update_state_display(player_state)

	# Build options from context
	_build_options(context)

	visible = true

	# Focus first button
	await get_tree().process_frame
	var first_button := options_container.get_child(0) as Button
	if first_button:
		first_button.grab_focus()


func hide_decision() -> void:
	visible = false
	_decision_context = {}


## Update state label with translated player state
func _update_state_display(state: String) -> void:
	if state.is_empty():
		state_label.text = ""
		state_label.visible = false
		return

	# Translate state to Korean display text
	var display_text: String = STATE_DISPLAY.get(state, state)
	state_label.text = "상태: %s" % display_text
	state_label.visible = true

	# Color based on state
	match state:
		"WithBall":
			state_label.add_theme_color_override("font_color", Color(1.0, 0.84, 0.0))  # Gold
		"Attacking":
			state_label.add_theme_color_override("font_color", Color(0.2, 0.8, 0.2))  # Green
		"Defending":
			state_label.add_theme_color_override("font_color", Color(0.4, 0.6, 1.0))  # Blue
		_:
			state_label.remove_theme_color_override("font_color")


func _build_options(context: Dictionary) -> void:
	# Clear existing options
	for child in options_container.get_children():
		child.queue_free()

	# Get options from context or build default options
	var options: Array = context.get("options", [])

	if options.is_empty():
		# Default options when engine doesn't provide specific ones
		options = _get_default_options(context)

	# Create buttons for each option
	for opt in options:
		if not (opt is Dictionary):
			continue
		_create_option_button(opt)


func _get_default_options(context: Dictionary) -> Array:
	## Build default options based on position and game state
	var options: Array = []

	# Position for context-aware options
	var pos_x: float = context.get("position_x", 0.5)
	var pos_y: float = context.get("position_y", 0.5)

	# Near goal? Emphasize shooting
	var near_goal: bool = pos_x > 0.75

	# 1. Shoot option
	var shoot_risk := "보통"
	var shoot_success := 40
	if near_goal:
		shoot_success = 65
		shoot_risk = "낮음"

	(
		options
		. append(
			{
				"action": "shoot",
				"label": "🥅 슈팅",
				"description": "골대를 향해 슈팅",
				"success_rate": shoot_success,
				"risk": shoot_risk,
			}
		)
	)

	# 2. Pass options (simplified - could be expanded with specific targets)
	(
		options
		. append(
			{
				"action": "pass",
				"label": "⚽ 패스",
				"description": "가장 좋은 위치의 동료에게 패스",
				"success_rate": 75,
				"risk": "낮음",
			}
		)
	)

	# 3. Dribble option
	var dribble_success := 55
	var dribble_risk := "높음"
	if not near_goal:
		dribble_success = 70
		dribble_risk = "보통"

	(
		options
		. append(
			{
				"action": "dribble",
				"label": "🏃 드리블",
				"description": "수비수를 돌파하며 전진",
				"success_rate": dribble_success,
				"risk": dribble_risk,
			}
		)
	)

	# 4. Safe pass (back pass)
	(
		options
		. append(
			{
				"action": "safe_pass",
				"label": "↩️ 안전 패스",
				"description": "후방으로 안전하게 볼 연결",
				"success_rate": 90,
				"risk": "매우 낮음",
			}
		)
	)

	return options


func _create_option_button(opt: Dictionary) -> void:
	var container := HBoxContainer.new()
	container.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	# Main button
	var button := Button.new()
	button.text = opt.get("label", "Action")
	button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	button.custom_minimum_size = Vector2(200, 50)

	# Add tooltip with description
	var desc: String = opt.get("description", "")
	var success: int = int(opt.get("success_rate", 50))
	var risk: String = opt.get("risk", "보통")
	button.tooltip_text = "%s\n성공률: %d%%\n리스크: %s" % [desc, success, risk]

	# Connect button
	var action_data := opt.duplicate()
	button.pressed.connect(_on_option_pressed.bind(action_data))

	container.add_child(button)

	# Success rate label
	var rate_label := Label.new()
	rate_label.text = "%d%%" % success
	rate_label.custom_minimum_size = Vector2(60, 0)
	rate_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER

	# Color based on success rate
	if success >= 70:
		rate_label.add_theme_color_override("font_color", Color.GREEN)
	elif success >= 50:
		rate_label.add_theme_color_override("font_color", Color.YELLOW)
	else:
		rate_label.add_theme_color_override("font_color", Color.ORANGE)

	container.add_child(rate_label)

	# Risk label
	var risk_label := Label.new()
	risk_label.text = risk
	risk_label.custom_minimum_size = Vector2(80, 0)
	risk_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER

	match risk:
		"매우 낮음", "낮음":
			risk_label.add_theme_color_override("font_color", Color.GREEN)
		"보통":
			risk_label.add_theme_color_override("font_color", Color.YELLOW)
		"높음", "매우 높음":
			risk_label.add_theme_color_override("font_color", Color.RED)

	container.add_child(risk_label)

	options_container.add_child(container)


func _on_option_pressed(action_data: Dictionary) -> void:
	print("[PlayerCommandOverlay] Action selected: %s" % action_data.get("action", "unknown"))

	# Build action dictionary for engine
	var action: Dictionary = {
		"action": action_data.get("action", "pass"),
	}

	# Add target if available
	if action_data.has("target_id"):
		action["target_id"] = action_data.get("target_id")

	# Hide overlay
	hide_decision()

	# Emit signal for parent to handle
	action_selected.emit(action)

	# If controller is available, submit directly
	if _session_controller and _session_controller.has_method("resume_from_hero_time"):
		_session_controller.resume_from_hero_time(action)
